use std::{error::Error, fs::File, io::Read, net::TcpListener, rc::Rc, thread};

use websocket::{
    Message,
    native_tls::{Identity, TlsAcceptor},
    server::WsServer,
    sync::Server,
};

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
                    let mut client = stream.accept().unwrap();
                    let message = Message::text("Server sent: Hello, client!");
                    let _ = client.send_message(&message);

                    loop {
                        match client.recv_message() {
                            Ok(msg) => {
                                if msg.is_data() {
                                    println!("Client sent: {msg:?}");
                                }
                            }

                            Err(e) => {
                                if !e.to_string().contains("NoDataAvailable") {
                                    return;
                                }
                                println!("Received invalid message: {e:?}");
                            }
                        }
                    }
                });
            }

            Err(e) => {
                eprintln!("Could not connect client: {e:?}");
            }
        }
    }
}
