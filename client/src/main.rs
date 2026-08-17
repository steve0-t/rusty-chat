use std::{
    collections::HashMap,
    io::{self, BufRead},
    sync::Arc,
};

use anyhow::Result;

use secret_service::SecretService;
use tokio::net::TcpStream;
use tokio_rustls::rustls::{
    ClientConfig, RootCertStore,
    pki_types::{CertificateDer, pem::PemObject},
};

use futures_util::{self, SinkExt, StreamExt, stream::SplitSink};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream,
    tungstenite::{Message, Utf8Bytes},
};
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<()> {
    let session_token = get_session_token().await;

    let ws_stream = establish_ws_connection().await?;

    let (mut write, mut read) = ws_stream.split();

    let access_token = if session_token.is_none() {
        request_session(&mut write).await;
    } else {
        let session_token = session_token.unwrap();
        restore_session(session_token, &mut write).await;
    };

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

async fn establish_ws_connection() -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>> {
    let certs: Vec<_> = CertificateDer::pem_file_iter("rootCA.pem")
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

    let (ws_stream, _response) = tokio_tungstenite::connect_async_tls_with_config(
        "wss://127.0.0.1:9090",
        None,
        false,
        Some(connector),
    )
    .await?;

    return Ok(ws_stream);
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
) {
}
