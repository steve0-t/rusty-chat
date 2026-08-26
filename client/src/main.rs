use std::{
    collections::HashMap,
    io::{self, BufRead},
    sync::Arc,
    thread,
    time::Duration,
};

use anyhow::{Result, anyhow};

use secret_service::SecretService;
use tokio::{
    net::TcpStream,
    sync::mpsc::{self, Receiver, channel},
};
use tokio_rustls::rustls::{
    ClientConfig, RootCertStore,
    pki_types::{CertificateDer, pem::PemObject},
};

use futures_util::{self, SinkExt, StreamExt, stream::SplitSink, stream::SplitStream};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream,
    tungstenite::{Connector, Message, Utf8Bytes},
};
use tokio_util::sync::CancellationToken;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::Rect,
    macros::ratatui_core,
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
};

pub const SERVER_IP: &str = "wss://127.0.0.1:9090";

#[derive(Debug, Default)]
pub struct App {
    exit: bool,
}

impl App {
    /// runs the application's main loop until the user quits
    fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::poll(Duration::from_millis(50)) {
            Ok(true) => match event::read() {
                Ok(e) => {
                    if e.is_key_press() {
                        self.handle_key_event(&e.as_key_event().unwrap())
                    }
                }
                Err(e) => {
                    eprintln!("Failed to capture key event: {e:?}");
                }
            },
            _ => {}
        }
        Ok(())
    }

    fn handle_key_event(&self, e: &KeyEvent) {
        todo!()
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(" Chat app ".bold());

        let instructions = Line::from(vec![
            // Text on the bottom
            " Quit ".into(),
            "<Q> ".blue().bold(),
        ]);

        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK)
            .render(area, buf);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // let session_token = get_session_token().await;
    let (stream, sink) = channel::<String>(5);

    let networking_thread = thread::spawn(|| {
        networking_runtime(sink);
    });

    // let access_token = match session_token {
    //     Some(session_token) => restore_session(session_token, &mut write, &mut read).await?,

    //     None => request_session(&mut write).await?,
    // };

    ratatui::run(|terminal| App::default().run(terminal))
}

#[tokio::main]
async fn networking_runtime(sink: Receiver<String>) {
    loop {
        // println!("CLient: establishing connection to server...");
        let _ = handle_connection().await;
        tokio::time::sleep(Duration::from_millis(1500)).await;
    }
}

async fn handle_connection() -> Result<()> {
    let ws_stream = establish_ws_connection().await?;
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
