use std::{
    error::Error,
    fs::File,
    io::{self, Read},
    net::{TcpListener, TcpStream},
    ops::Sub,
    process::Command,
    rc::Rc,
    thread,
};

use surrealdb::{Surreal, opt::EndpointKind::SurrealKv};
use websocket::{
    Message,
    header::CacheDirective::Private,
    native_tls::{Identity, TlsAcceptor, TlsStream},
    server::{WsServer, upgrade::WsUpgrade},
    sync::{Server, server::upgrade::Buffer},
};

enum CommandType {
    CreateChannel = 2,
    Quit = 1,
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

fn main() -> Result<(), Box<dyn Error>> {
    let mut cert_file = match File::open("cert.pem") {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Could not open certificate file: {e:?}");
            return Err(e.into());
        }
    };

    let mut cert_buf = vec![];

    cert_file.read_to_end(&mut cert_buf)?;

    let mut cert_key_file = match File::open("key.pem") {
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

    let mut server = match Server::bind_secure("127.0.0.1:9090", tls_acceptor) {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Failed to bind server: {e:?}");
            return Err(e.into());
        }
    };

    deploy_server(&mut server);

    Ok(())
}

fn deploy_server(server: &mut WsServer<TlsAcceptor, TcpListener>) {
    while let Some(conn) = server.next() {
        match conn {
            Ok(stream) => {
                thread::spawn(move || {
                    handle_client(stream);
                });
            }

            Err(e) => {
                eprintln!("Could not connect client: {e:?}");
            }
        }
    }
}

fn handle_client(stream: WsUpgrade<TlsStream<TcpStream>, Option<Buffer>>) {
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
                            // println!("{op}");
                            let cmd = CommandType::try_from(op);
                            match cmd {
                                Ok(CommandType::CreateChannel) => {
                                    print!("Name of channel: ");
                                    let mut buf = String::new();
                                    let res = io::stdin().read_line(&mut buf);
                                    match res {
                                        Ok(n) => {
                                            todo!();
                                        }
                                        Err(e) => {
                                            eprintln!("Failed to get new channel name: {e}");
                                        }
                                    }
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
                    return;
                }
                websocket::OwnedMessage::Ping(_items) => todo!(),
                websocket::OwnedMessage::Pong(_items) => todo!(),
            },

            Err(websocket::WebSocketError::NoDataAvailable) => {
                println!("Client disconnected");
                return;
            }

            Err(e) => {
                println!("Received invalid message: {e:?}");
                return;
            }
        }
    }
}
