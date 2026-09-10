use std::{
    collections::HashMap,
    fs::File,
    io::{self, BufRead, Read},
    sync::Arc,
    thread,
    time::Duration,
};

use anyhow::Result;

use ratatui::{Terminal, backend::CrosstermBackend};
use secret_service::SecretService;
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

    let networking_thread = thread::spawn(|| {
        networking_runtime(receiver);
    });

    let access_token = match get_session_token().await {
        Some(session_token) => restore_session(session_token, &mut write, &mut read).await?,
        None => request_session(&mut write).await?,
    };

    let last_logged_user = get_last_logged_user()?;

    let db = timeout(
        Duration::from_secs(5),
        surrealdb::engine::any::connect("ws://localhost:8000"),
    )
    .await??;
    db.use_ns("chat_app_local").use_db(last_logged_user).await?;

    // create app instance
    let mut app = app::App::new();

    // initialize the terminal user interface.
    let backend = CrosstermBackend::new(std::io::stderr());
    let terminal = Terminal::new(backend)?;
    let events = EventHandler::new(250);
    let mut tui = Tui::new(terminal, events);
    tui.enter()?;

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
async fn networking_runtime(receiver: Receiver<String>) {
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
        let _ = handle_connection(connector.clone()).await;
        tokio::time::sleep(Duration::from_millis(1500)).await;
    }
}

/// generates read and write handles for new connection
async fn handle_connection(connector: Connector) -> Result<()> {
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

    let _ = tokio::try_join!(read_handle, write_handle);
    Ok(())
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
    read_sink: &mut SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
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
    write_sink: &mut SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
) -> Result<()> {
    Ok(())
}

async fn sync_channels() -> Option<Vec<ChannelSelect>> {}

fn get_last_logged_user() -> Result<String> {
    let mut file = match File::open("users/last_logged_in") {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Could not open certificate file: {e:?}");
            return Err(e.into());
        }
    };

    let mut user = String::with_capacity(256 as usize);
    file.read_to_string(&mut user)?;

    Ok(user)
}
