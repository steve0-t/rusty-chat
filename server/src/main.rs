use core::error;
use std::{
    clone,
    fs::File,
    io::{self, Read},
    net::{TcpListener, TcpStream},
    ops::Sub,
    process::{Command, exit},
    rc::Rc,
    thread,
    time::Duration,
};

use surrealdb::{
    Surreal,
    engine::{
        any::Any,
        remote::ws::{Client, Ws},
    },
    opt::EndpointKind::SurrealKv,
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

use surrealdb_types::{RecordId, SurrealValue, Value};

use anyhow::{Error, Result, anyhow};

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
struct User {
    username: String,
    user_id: RecordId,
    created_at: Datetime,
    phone_number: Option<u32>,
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

    let db = timeout(
        Duration::from_secs(5),
        surrealdb::engine::any::connect("ws://localhost:8000"),
    )
    .await??;
    db.use_ns("main").use_db("main").await?;

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
        Some(_) => {
            return client_loop(db, &mut client).await;
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
                                    create_channel(&db, client).await;
                                }
                                Ok(CommandType::Quit) => {
                                    let message = Message::text("Bye client from server!");
                                    let _ = client.send_message(&message);
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
) {
    let _ = client.send_message(&Message::text("Name of channel: "));

    // let mut buf = String::new();
    match client.recv_message() {
        Ok(response) => {
            match response {
                OwnedMessage::Text(text) => {
                    // db.create("channel").content(Channel {
                    //     channel_name: buf,
                    //     owner: RecordId::new(table, key),
                    //     members: None,
                    //     created_at: Datetime::now(),
                    // });
                    println!("Created channel {text}");
                }
                OwnedMessage::Binary(_items) => todo!(),
                OwnedMessage::Close(_close_data) => todo!(),
                OwnedMessage::Ping(_items) => todo!(),
                OwnedMessage::Pong(_items) => todo!(),
            }
        }
        Err(e) => {
            eprintln!("Failed to get new channel name: {e}");
        }
    }
}

async fn auth_client(
    db: &Surreal<Any>,
    client: &mut websocket::client::sync::Client<TlsStream<TcpStream>>,
) -> Option<User> {
    let _ = client.send_message(&Message::text(
        "
            1. Log in
            2. Register
        ",
    ));
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
            let _ = client.send_message(&Message::text("Invalid method, aborting."));
            return None;
        }
    }
}

async fn log_in(
    db: &Surreal<Any>,
    client: &mut websocket::client::sync::Client<TlsStream<TcpStream>>,
) -> Option<User> {
    let _ = client.send_message(&Message::text("Username: "));
    let username = client.recv_message().ok()?;

    let _ = client.send_message(&Message::text("Password: "));
    let pswd = client.recv_message().ok()?;

    let (OwnedMessage::Text(username), OwnedMessage::Text(pswd)) = (username, pswd) else {
        return None;
    };

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
        .await
        .ok()?;

    let mut response = user.check().ok()?;
    let user: Option<User> = response.take(0).ok()?;
    match user {
        Some(user) => Some(user),
        None => {
            eprintln!("Could not find user '{username}'");
            None
        }
    }
}

async fn register(
    db: &Surreal<Any>,
    client: &mut websocket::client::sync::Client<TlsStream<TcpStream>>,
) -> Option<User> {
    return None;
}
