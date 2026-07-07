use std::{
    fs::{self, File},
    io::{BufReader, Read},
    net::TcpStream,
    sync::Arc,
};

use anyhow::{Context, Result};

use tokio_rustls::rustls::{
    ClientConfig, RootCertStore,
    pki_types::{CertificateDer, pem::PemObject},
};

#[tokio::main]
async fn main() -> Result<()> {
    let reader = File::open("cert.pem")?;

    let cert = CertificateDer::from_pem(
        tokio_rustls::rustls::pki_types::pem::SectionKind::Certificate,
        cert_file,
    )
    .with_context(|| format!("Failed to read certificate"))?;

    let mut root_cert_store = RootCertStore::empty();

    match root_cert_store.add(cert) {
        Ok(()) => (),
        Err(e) => {
            eprintln!("Failed to add certificate to store: {e:?}");
        }
    };

    let config = ClientConfig::builder()
        .with_root_certificates(root_cert_store)
        .with_no_client_auth();

    let connector = tokio_tungstenite::Connector::Rustls(Arc::new(config));

    tokio_tungstenite::connect_async_tls_with_config(
        "wss://127.0.0.1:9090",
        None,
        false,
        Some(connector),
    )
    .await?;

    Ok(())
}
