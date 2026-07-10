use std::{
    fs::{self, File},
    io::{self, BufRead, BufReader, Read},
    net::TcpStream,
    sync::Arc,
};

use anyhow::{Context, Result};

use tokio_rustls::rustls::{
    ClientConfig, RootCertStore,
    client::AlwaysResolvesClientRawPublicKeys,
    pki_types::{CertificateDer, pem::PemObject},
};

use futures_util::{self, SinkExt, StreamExt, TryStreamExt};
use tokio_tungstenite::tungstenite::{Message, Utf8Bytes};

#[tokio::main]
async fn main() -> Result<()> {
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

    let (mut ws_stream, response) = tokio_tungstenite::connect_async_tls_with_config(
        "wss://127.0.0.1:9090",
        None,
        false,
        Some(connector),
    )
    .await?;

    let (mut write, mut read) = ws_stream.split();

    let read_handle = tokio::spawn(async move {
        while let Some(msg) = read.next().await {
            match msg {
                Ok(msg) => {
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
            match line {
                Ok(msg) => match write.send(Message::text(msg)).await {
                    Ok(_) => {
                        println!("Client: successfully sent a message");
                    }
                    Err(e) => {
                        eprintln!("Client: failed to send a message: {e:?}");
                    }
                },
                Err(_) => todo!(),
            }
        }
    });

    let _ = tokio::try_join!(read_handle, write_handle);

    Ok(())
}
