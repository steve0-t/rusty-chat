use std::{
    error::Error,
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
use websocket::{
    Message,
    header::{CacheDirective::Private, RelationType::SuccessorVersion},
    native_tls::{Identity, TlsAcceptor, TlsStream},
    server::{WsServer, upgrade::WsUpgrade},
    sync::{Server, server::upgrade::Buffer},
};

use surrealdb_types::{RecordId, SurrealValue, Value};

use anyhow::Result;

enum CommandType {
    CreateChannel = 2,
    Quit = 1,
}

#[derive(Debug, SurrealValue)]
struct Channel {
    channel_name: String,
    owner: RecordId,
    members: Vec<RecordId>,
    created_at: Datetime,
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
                            eprintln!("Server: client errored out: {e:?}");
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
    let message = Message::text("Server sent: Hello, client!");
    let _ = client.send_message(&message);

    loop {
        match client.recv_message() {
            Ok(msg) => match msg {
                websocket::OwnedMessage::Text(text) => {
                    let op = text.trim().parse::<u32>();
                    match op {
                        Ok(op) => {
                            println!("{op}");
                            let cmd = CommandType::try_from(op);
                            match cmd {
                                Ok(CommandType::CreateChannel) => {
                                    create_channel(&db, &mut client).await;
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
                websocket::OwnedMessage::Binary(_items) => todo!(),
                websocket::OwnedMessage::Close(close_data) => {
                    match close_data {
                        Some(close_data) => {
                            println!("Client closed with: {close_data:?}");
                        }
                        None => {}
                    }
                    return Ok(());
                }
                websocket::OwnedMessage::Ping(_items) => todo!(),
                websocket::OwnedMessage::Pong(_items) => todo!(),
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
            // db.create("channel").content(Channel {
            //     channel_name: buf,
            //     owner: RecordId::new(table, key),
            //     members: None,
            //     created_at: Datetime::now(),
            // });
            match response {
                websocket::OwnedMessage::Text(text) => {
                    println!("Created channel {text}");
                }
                websocket::OwnedMessage::Binary(_items) => todo!(),
                websocket::OwnedMessage::Close(_close_data) => todo!(),
                websocket::OwnedMessage::Ping(_items) => todo!(),
                websocket::OwnedMessage::Pong(_items) => todo!(),
            }
        }
        Err(e) => {
            eprintln!("Failed to get new channel name: {e}");
        }
    }
}
