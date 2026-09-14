use std::{
    collections::HashMap,
    fs::File,
    io::{self, BufRead, Read},
    sync::Arc,
    thread::{self, JoinHandle},
    time::Duration,
};

use anyhow::Result;

use ratatui::{Terminal, backend::CrosstermBackend};
use secret_service::SecretService;
use surrealdb::{Surreal, engine::any::Any};
use tokio::{
    net::TcpStream,
    sync::mpsc::{Receiver, channel},
    time::timeout,
};
use tokio_rustls::rustls::{
    ClientConfig, RootCertStore,
    pki_types::{CertificateDer, pem::PemObject},
};

use futures_util::{self, SinkExt, StreamExt, stream::SplitSink, stream::SplitStream};
use tokio_tungstenite::{
    Connector, MaybeTlsStream, WebSocketStream,
    tungstenite::{Message, Utf8Bytes},
};
use tokio_util::sync::CancellationToken;

use crate::{
    app_event::{AppEvent, EventHandler},
    tui::Tui,
};

pub const SERVER_IP: &str = "wss://127.0.0.1:9090";

type ReadHandle = JoinHandle<()>;
type WriteHandle = JoinHandle<()>;

pub mod app;
pub mod app_event;
pub mod tui;
pub mod ui;
pub mod update;

pub enum RequestData {
    Username = 1,
}

#[tokio::main]
async fn main() -> Result<()> {
    // let session_token = get_session_token().await;
    let (sender, receiver) = channel::<String>(5);

    // let access_token = match get_session_token().await {
    //     Some(session_token) => restore_session(session_token, &mut write, &mut read).await?,
    //     None => request_session(&mut write).await?,
    // };

    // connect to db service
    let db = timeout(
        Duration::from_secs(5),
        surrealdb::engine::any::connect("ws://localhost:8000"),
    )
    .await??;

    // create app instance
    let mut app = Arc::from(app::App::new());

    let networking_thread = thread::spawn(|| {
        networking_runtime(receiver, app.clone());
    });

    // initialize the terminal user interface.
    let backend = CrosstermBackend::new(std::io::stderr());
    let terminal = Terminal::new(backend)?;
    let events = EventHandler::new(250);
    let mut tui = Tui::new(terminal, events);
    tui.enter()?;

    // try to get last logged user
    // if some, use their db
    // else auth user
    let user = get_last_logged_user();
    if user.is_none() {
        let user = log_in(&db).await;

        if user.is_none() {
            app.failed_to_get_user = true;
        }
    }
    db.use_ns("chat_app_local").use_db(user).await?;

    // start the main loop.
    while !app.should_quit {


        // Render the user interface.
        tui.draw(&mut app)?;

        // Handle events.
        match tui.events.next()? {
            AppEvent::Tick => {}
            AppEvent::Key(key_event) => update::update(&mut app, key_event),
            AppEvent::Mouse(_) => {}
            AppEvent::Resize(newX, newY) => {}
        };
    }

    // exit the user interface.
    tui.exit()?;
    Ok(())
}

/// loop for networking thread
#[tokio::main]
async fn networking_runtime(receiver: Receiver<String>, app: Arc<app::App>) {
    let certs: Vec<_> = CertificateDer::pem_file_iter("certs/rootCA.pem")
        .unwrap()
        .collect();

    let mut root_cert_store = RootCertStore::empty();

    for cert in certs {
        match cert {
            Ok(cert) => {
                match root_cert_store.add(cert) {
                    Ok(()) => (),
                    Err(e) => {
                        eprintln!("Failed to add certificate to store: {e:?}");
                    }
                };
            }
            Err(e) => {
                eprintln!("Failed to add certificate: {e:?}");
            }
        }
    }

    let config = ClientConfig::builder()
        .with_root_certificates(root_cert_store)
        .with_no_client_auth();

    let connector = tokio_tungstenite::Connector::Rustls(Arc::new(config));

    loop {
        // println!("CLient: establishing connection to server...");
        let (read_handle, write_handle) = match handle_connection(connector.clone()).await {
            Ok(res) => res,
            Err(e) => {
                app.connection_available = false;
                return ();
            }
        }
        tokio::time::sleep(Duration::from_millis(1500)).await;
    }
}

/// generates read and write handles for new connection
async fn handle_connection(connector: Connector) -> Result<(ReadHandle, WriteHandle)> {
    let ws_stream = establish_ws_connection(connector).await?;
    let (mut write, mut read) = ws_stream.split();

    let token = CancellationToken::new();

    let cloned_token = token.clone();

    let read_handle = tokio::spawn(async move {
        while let Some(msg) = read.next().await {
            match msg {
                Ok(msg) => {
                    if msg.is_close() {
                        token.cancel();
                        return;
                    }
                    println!("Client: Received message: {msg}");
                }
                Err(e) => {
                    eprintln!("Client: Failed to receive message: {e:?}");
                }
            }
        }
    });

    let mut reader = io::BufReader::new(io::stdin()).lines();
    let write_handle = tokio::spawn(async move {
        while let Some(line) = reader.next() {
            if cloned_token.is_cancelled() {
                return;
            }

            match line {
                Ok(msg) => match write.send(Message::text(msg)).await {
                    Ok(_) => {
                        println!("Client: successfully sent a message");
                    }
                    Err(e) => {
                        if e.to_string().contains("Broken pipe") {
                            break;
                        }
                        eprintln!("Client: failed to send a message: {e:?}");
                    }
                },
                Err(e) => {
                    eprintln!("Client: failed to read input: {e:?}");
                }
            }
        }
    });

    Ok([read_handle, write_handle])
}

/// TODO write description
async fn establish_ws_connection(
    connector: Connector,
) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>> {
    // let (ws_stream, _response) =
    let (ws_stream, _resp) =
        tokio_tungstenite::connect_async_tls_with_config(SERVER_IP, None, false, Some(connector))
            .await?;

    Ok(ws_stream)
}

async fn get_session_token() -> Option<Vec<u8>> {
    let ss = match SecretService::connect(secret_service::EncryptionType::Dh).await {
        Ok(res) => res,
        Err(secret_service::Error::Unavailable) => {
            eprintln!("No secret service found");
            return None;
        }
        Err(e) => {
            eprintln!("Failed to retrieve session token: {e:?}");
            return None;
        }
    };

    // let collection = ss.get_default_collection().await?;

    let search_items = ss
        .search_items(HashMap::from([("app", "chat_app"), ("key", "session_key")]))
        .await
        .ok()?;

    let session_token = match search_items.unlocked.first() {
        Some(item) => item,
        None => {
            let locked_item = search_items
                .locked
                .first()
                .expect("Search didn't return any items!");
            locked_item.unlock().await.unwrap();
            locked_item
        }
    };
    session_token.get_secret().await.ok()
}

async fn restore_session(
    token: Vec<u8>,
    write_sink: &mut SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    _read_sink: &mut SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
) -> Result<()> {
    match write_sink
        .send(Message::text(Utf8Bytes::try_from(token)?))
        .await
    {
        Ok(_) => {}

        Err(e) => {
            eprintln!("Client: {e}");
            return Err(e.into());
        }
    }

    Ok(())
}

async fn request_session(
    _write_sink: &mut SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
) -> Result<()> {
    Ok(())
}

// async fn sync_channels() -> Option<Vec<ChannelSelect>> {}

fn get_last_logged_user() -> Option<String> {
    let mut file = match File::open("users/last_logged_in") {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Could not open certificate file: {e:?}");
            return None;
        }
    };

    let mut user = String::with_capacity(256 as usize);
    file.read_to_string(&mut user).ok()?;

    Some(user)
}

async fn log_in(db: &Surreal<Any>) -> Option<String> {
    let [username, pswd] = app.get_input_from_user(["Username: ", "Password: "]).ok()?;

    let (OwnedMessage::Text(username), OwnedMessage::Text(pswd)) = (username, pswd) else {
        return None;
    };

    // println!("{username} {pswd}");
    // println!("getting user by username");

    let user = get_user::<UserInsert>(&username, db).await;

    // println!("verifying user");
    if let Some(user) = user {
        let Ok(pswd_hash) = PasswordHash::new(&user.password) else {
            return None;
        };

        let argon2 = Argon2::default();

        match argon2.verify_password(&pswd.into_bytes(), &pswd_hash) {
            Ok(_) => Some(user.username),
            Err(e) => {
                eprintln!("Failed to verify user: {e:?}");
                None
            }
        }
    } else {
        eprintln!("Server: failed to fetch user");
        None
    }
}

async fn register(db: &Surreal<Any>) -> Option<String> {
    let [username, pswd, repeat_pswd] =
        get_input_from_user(["Username: ", "Password: ", "Repeat password: "]).ok()?;

    let (OwnedMessage::Text(username), OwnedMessage::Text(pswd), OwnedMessage::Text(repeat_pswd)) =
        (username, pswd, repeat_pswd)
    else {
        return None;
    };

    if pswd.len() != repeat_pswd.len() || pswd != repeat_pswd {
        send_msg_to_client(&"Entered passwords do not match".to_string(), client);
        return None;
    }

    let user = get_user::<UserSelect>(&username, db).await;

    match user {
        Some(_) => {
            let _ = send_msg_to_client(&"User '{username}' already exists".to_string(), client);
            return None;
        }
        None => {}
    };

    send_msg_to_client(&"Enter phone number (optional): ".to_string(), client);
    let phone_number = match client.recv_message().ok()? {
        OwnedMessage::Text(text) => Some(text),
        _ => None,
    };

    send_msg_to_client(&"Enter email address (optional): ".to_string(), client);
    let email_addr = match client.recv_message().ok()? {
        OwnedMessage::Text(text) => Some(text),
        _ => None,
    };

    let salt = SaltString::generate(&mut OsRng);

    let argon2 = Argon2::default();

    let hashed_pass = argon2
        .hash_password(&pswd.into_bytes(), &salt)
        .ok()?
        .to_string();

    match db
        .insert::<Option<UserSelect>>(("users", Uuid::new_v7()))
        .content(UserInsert {
            username: username,
            password: hashed_pass,
            created_at: Datetime::now(),
            phone_number: phone_number,
            email_addr: email_addr,
        })
        .await
    {
        Ok(user) => user.map(|u| u.username),

        Err(e) => {
            eprintln!("Failed to register user: {e:?}");
            None
        }
    }
}
