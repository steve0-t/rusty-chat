use core::error;
use std::{
    clone,
    fs::File,
    io::{self, ErrorKind::ArgumentListTooLong, Read},
    net::{TcpListener, TcpStream},
    ops::Sub,
    process::{Command, exit},
    rc::Rc,
    thread,
    time::Duration,
    vec,
};

use argon2::{
    Algorithm::Argon2id,
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{PasswordHashString, SaltString, rand_core::OsRng},
};
use surrealdb::{
    Surreal,
    engine::{
        any::Any,
        remote::ws::{Client, Ws},
    },
    opt::{EndpointKind::SurrealKv, auth::Root},
    types::{Datetime, Uuid},
};

use tokio::time::timeout;
use tokio_rustls::rustls::{HandshakeType::ClientHello, pki_types::pem::SectionKind::PrivateKey};
use websocket::{
    Message, OwnedMessage,
    header::{CacheDirective::Private, Preference::ReturnMinimal, RelationType::SuccessorVersion},
    native_tls::{Identity, TlsAcceptor, TlsStream},
    server::{WsServer, upgrade::WsUpgrade},
    sync::{Server, server::upgrade::Buffer},
};

use surrealdb_types::{RecordId, SurrealValue, Value, uuid};

use anyhow::{Error, Result, anyhow};

// macro_rules! strif {
//     ( $( $x:expr ),* ) => {
//         {
//             let mut temp_vec = Vec::new();
//             $(
//                 temp_vec.push($x);
//             )*
//             temp_vec
//         }
//     };
// }

enum CommandType {
    Quit = 1,
    CreateChannel = 2,
}

#[derive(Debug, SurrealValue)]
struct Channel {
    channel_name: String,
    owner: Option<RecordId>,
    members: Vec<RecordId>,
    created_at: Datetime,
}

#[derive(Debug, SurrealValue)]
struct UserInsert {
    username: String,
    password: String,
    created_at: Datetime,
    phone_number: Option<String>,
    email_addr: Option<String>,
}

#[derive(Debug, SurrealValue)]
struct UserSelect {
    id: RecordId,
    username: String,
    created_at: Datetime,
    phone_number: Option<String>,
    email_addr: Option<String>,
}

impl TryFrom<u32> for CommandType {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            x if x == CommandType::Quit as u32 => Ok(CommandType::Quit),
            x if x == CommandType::CreateChannel as u32 => Ok(CommandType::CreateChannel),
            // x if x == CommandType::C as i32 => Ok(CommandType::C),
            _ => Err(()),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut cert_file = match File::open("rustychat.com+4.pem") {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Could not open certificate file: {e:?}");
            return Err(e.into());
        }
    };

    let mut cert_buf = vec![];

    cert_file.read_to_end(&mut cert_buf)?;

    let mut cert_key_file = match File::open("rustychat.com+4-key.pem") {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Could not open certificate key file: {e:?}");
            return Err(e.into());
        }
    };

    let mut cert_key = vec![];
    cert_key_file.read_to_end(&mut cert_key)?;

    let ident = match Identity::from_pkcs8(&cert_buf, &cert_key) {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Failed to load certificate: {e:?}");
            return Err(e.into());
        }
    };

    let tls_acceptor = TlsAcceptor::builder(ident)
        .build()
        .expect("Failed to create TLS acceptor");

    let db = timeout(
        Duration::from_secs(5),
        surrealdb::engine::any::connect("ws://localhost:8000"),
    )
    .await??;

    db.signin(Root {
        username: "chat_app_owner".to_string(),
        password: "security".to_string(),
    })
    .await?;

    db.use_ns("chat_app").use_db("production").await?;

    let addr = "127.0.0.1:9090".to_string();
    let mut server = match Server::bind_secure(&addr, tls_acceptor) {
        Ok(server) => {
            println!("Server: Ready! Listening on {addr}");
            server
        }
        Err(e) => {
            eprintln!("Failed to bind server: {e:?}");
            return Err(e.into());
        }
    };

    match deploy_server(&mut server, &db).await {
        Ok(()) => Ok(()),
        Err(e) => Err(e),
    }
}

async fn deploy_server(
    server: &mut WsServer<TlsAcceptor, TcpListener>,
    db: &Surreal<Any>,
) -> Result<()> {
    while let Some(conn) = server.next() {
        match conn {
            Ok(stream) => {
                let db = db.clone();
                println!("New client connected!");
                tokio::spawn(async move {
                    match handle_client(stream, &db).await {
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!("Server: client closed with error: {e:?}");
                        }
                    }
                });
            }

            Err(e) => {
                eprintln!("Could not connect client: {e:?}");
            }
        }
    }
    Ok(())
}

async fn handle_client(
    stream: WsUpgrade<TlsStream<TcpStream>, Option<Buffer>>,
    db: &Surreal<Any>,
) -> Result<()> {
    let mut client = stream.accept().unwrap();

    match auth_client(&db, &mut client).await {
        Some(username) => {
            send_message_to_client(
                &format!("Successfully logged in as {}", username).to_string(),
                &mut client,
            );
            return client_loop(db, &mut client, &username.as_str()).await;
        }
        None => {
            let _ = client.send_message(&Message::close());
            let _ = client.shutdown();
            return Err(anyhow!("Failed to authenticate client"));
        }
    };
}

async fn client_loop(
    db: &Surreal<Any>,
    client: &mut websocket::client::sync::Client<TlsStream<TcpStream>>,
    username: &str,
) -> Result<()> {
    loop {
        match client.recv_message() {
            Ok(msg) => match msg {
                OwnedMessage::Text(text) => {
                    let op = text.trim().parse::<u32>();
                    match op {
                        Ok(op) => {
                            println!("{op}");
                            let cmd = CommandType::try_from(op);
                            match cmd {
                                Ok(CommandType::CreateChannel) => {
                                    // create_channel(&db, client).await;
                                    let _ = create_channel(&db, client, username, Vec::new()).await;
                                }
                                Ok(CommandType::Quit) => {
                                    let _ = client
                                        .send_message(&Message::text("Bye client from server!"));
                                    let _ = client.shutdown();
                                }
                                Err(e) => {
                                    eprintln!("Unknown command: {e:?}");
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Encountered error when parsing operation request: {e}");
                        }
                    }
                    println!("Received from client: {:?}", text);
                }
                OwnedMessage::Binary(_items) => todo!(),
                OwnedMessage::Close(close_data) => {
                    match close_data {
                        Some(close_data) => {
                            println!("Client closed with: {close_data:?}");
                        }
                        None => {}
                    }
                    return Ok(());
                }
                OwnedMessage::Ping(_items) => todo!(),
                OwnedMessage::Pong(_items) => todo!(),
            },

            Err(websocket::WebSocketError::NoDataAvailable) => {
                println!("Client disconnected");
                return Ok(());
            }

            Err(e) => {
                println!("Received invalid message: {e:?}");
                return Err(e.into());
            }
        }
    }
}

async fn create_channel(
    db: &Surreal<Any>,
    client: &mut websocket::client::sync::Client<TlsStream<TcpStream>>,
    username: &str,
    members: Vec<RecordId>,
) -> Result<()> {
    send_message_to_client(&"Name of channel: ".to_string(), client);

    // let mut buf = String::new();
    let Ok(OwnedMessage::Text(channel_name)) = client.recv_message() else {
        return Err(anyhow!("Failed to create channel"));
    };

    // println!("getting user by username");
    let mut user = db
        .query(
            "
                SELECT * FROM users
                WHERE username = $username
            ",
        )
        .bind(("username", username))
        .await?;

    let Some(user) = user.take::<Option<UserSelect>>(0)? else {
        return Err(anyhow!("Failed to retrieve user"));
    };

    match db
        .insert::<Vec<Channel>>("channels")
        .content(Channel {
            channel_name: channel_name,
            owner: Some(user.id),
            members: members,
            created_at: Datetime::now(),
        })
        .await
    {
        Ok(_) => Ok(()),
        Err(e) => Err(anyhow!(e)),
    }
}

async fn auth_client(
    db: &Surreal<Any>,
    client: &mut websocket::client::sync::Client<TlsStream<TcpStream>>,
) -> Option<String> {
    send_message_to_client(
        &"
            1. Log in
            2. Register
        "
        .to_string(),
        client,
    );
    let method = client.recv_message().ok()?;
    let OwnedMessage::Text(method) = method else {
        return None;
    };

    match method.parse::<u32>() {
        Ok(method) => {
            if method == 1 {
                log_in(db, client).await
            } else if method == 2 {
                register(db, client).await
            } else {
                return None;
            }
        }
        Err(e) => {
            eprintln!("User chose invalid method: {e:?}");
            send_message_to_client(&"Invalid method, aborting.".to_string(), client);
            return None;
        }
    }
}

async fn log_in(
    db: &Surreal<Any>,
    client: &mut websocket::client::sync::Client<TlsStream<TcpStream>>,
) -> Option<String> {
    send_message_to_client(&"Username: ".to_string(), client);
    let username = client.recv_message().ok()?;

    send_message_to_client(&"Password: ".to_string(), client);
    let pswd = client.recv_message().ok()?;

    let (OwnedMessage::Text(username), OwnedMessage::Text(pswd)) = (username, pswd) else {
        return None;
    };

    // println!("getting user by username");
    let user = db
        .query(
            "
                SELECT * FROM users
                WHERE username = $username
            ",
        )
        .bind(("username", username.as_str()))
        .await
        .ok()?;

    // println!("getting indexed results");
    let mut response = user.check().ok()?;

    // println!("getting first result");
    let user: Option<UserInsert> = response.take(0).ok()?;

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
        None
    }
}

async fn register(
    db: &Surreal<Any>,
    client: &mut websocket::client::sync::Client<TlsStream<TcpStream>>,
) -> Option<String> {
    send_message_to_client(&"Username: ".to_string(), client);
    let username = client.recv_message().ok()?;

    send_message_to_client(&"Password: ".to_string(), client);
    let pswd = client.recv_message().ok()?;

    send_message_to_client(&"Repeat password: ".to_string(), client);
    let repeat_pswd = client.recv_message().ok()?;

    let (OwnedMessage::Text(username), OwnedMessage::Text(pswd), OwnedMessage::Text(repeat_pswd)) =
        (username, pswd, repeat_pswd)
    else {
        return None;
    };

    if pswd.len() != repeat_pswd.len() || pswd != repeat_pswd {
        send_message_to_client(&"Entered passwords do not match".to_string(), client);
        return None;
    }

    let user = db
        .query(
            "
                SELECT username, password
                FROM users
                WHERE username = $username
                AND password = $password
            ",
        )
        .bind((("username", username.as_str()), ("password", pswd.as_str())))
        .await;

    match user {
        Ok(_) => {
            let _ = send_message_to_client(&"User '{username}' exists".to_string(), client);
            return None;
        }
        Err(_e) => {}
    };

    send_message_to_client(&"Enter phone number (optional): ".to_string(), client);
    let phone_number = match client.recv_message().ok()? {
        OwnedMessage::Text(text) => Some(text),
        _ => None,
    };

    send_message_to_client(&"Enter email address (optional): ".to_string(), client);
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

fn send_message_to_client(
    msg: &str,
    client: &mut websocket::client::sync::Client<TlsStream<TcpStream>>,
) {
    let _ = client.send_message(&Message::text(msg));
}
