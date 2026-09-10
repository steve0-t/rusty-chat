use core::error;
use std::{
    array, clone,
    fs::File,
    io::{self, ErrorKind::ArgumentListTooLong, Read},
    mem::MaybeUninit,
    net::{TcpListener, TcpStream},
    ops::{Index, Sub},
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
use rand::RngExt;
use sha2::{Digest, Sha512};
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
use tokio_rustls::rustls::{
    HandshakeType::ClientHello, crypto::hash::HashAlgorithm::SHA256,
    pki_types::pem::SectionKind::PrivateKey,
};
use websocket::{
    Message, OwnedMessage,
    header::{CacheDirective::Private, Preference::ReturnMinimal, RelationType::SuccessorVersion},
    native_tls::{Identity, TlsAcceptor, TlsStream},
    server::{WsServer, upgrade::WsUpgrade},
    sync::{Server, server::upgrade::Buffer},
};

use surrealdb_types::{RecordId, SurrealValue, Value, uuid};

use anyhow::{Error, Result, anyhow};

pub enum CommandType {
    Quit = 1,
    CreateChannel = 2,
    DisplayChannels = 3,
    OpenChannel = 4,
}

#[derive(Debug, SurrealValue)]
pub struct ChannelInsert {
    channel_name: String,
    owner: Option<RecordId>,
    members: Vec<RecordId>,
    created_at: Datetime,
}

#[derive(Debug, SurrealValue)]
pub struct ChannelSelect {
    id: RecordId,
    channel_name: String,
    owner: Option<RecordId>,
    members: Vec<RecordId>,
    created_at: Datetime,
}

#[derive(Debug, SurrealValue)]
pub struct UserInsert {
    username: String,
    password: String,
    created_at: Datetime,
    phone_number: Option<String>,
    email_addr: Option<String>,
}

#[derive(Debug, SurrealValue)]
pub struct UserSelect {
    id: RecordId,
    username: String,
    created_at: Datetime,
    phone_number: Option<String>,
    email_addr: Option<String>,
}

#[derive(Debug, SurrealValue)]
pub struct MessageType {
    sender: RecordId,
    channel: RecordId,
    sent_at: Datetime,
    status: String,
    content: Vec<String>,
}

#[derive(Debug, SurrealValue)]
pub struct SessionSelect {
    user_id: RecordId,
    token_hash: String,
    device_info: String,
    created_at: Datetime,
    expires_at: Datetime,
    last_used_at: Datetime,
    revoked_at: Datetime,
}

#[derive(Debug, SurrealValue)]
pub struct AccessToken(String);

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

pub const SERVER_IP: &str = "wss://127.0.0.1:9090";

#[tokio::main]
async fn main() -> Result<()> {
    let mut cert_file = match File::open("certs/rustychat.com+4.pem") {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Could not open certificate file: {e:?}");
            return Err(e.into());
        }
    };

    let mut cert_buf = vec![];
    cert_file.read_to_end(&mut cert_buf)?;

    let mut cert_key_file = match File::open("certs/rustychat.com+4-key.pem") {
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

    let addr = SERVER_IP;
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

    deploy_server(&mut server, &db).await
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

    let access_token = try_to_restore_session(db, &mut client).await;

    if access_token.is_some() {}

    match auth_client(&db, &mut client).await {
        Some(username) => {
            send_msg_to_client(
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

async fn try_to_restore_session(
    db: &Surreal<Any>,
    client: &mut websocket::client::sync::Client<TlsStream<TcpStream>>,
) -> Option<AccessToken> {
    match client.recv_message() {
        Ok(token) => {
            let OwnedMessage::Text(token) = token else {
                return None;
            };

            let hashed_token = match hash_token(&token) {
                Some(token) => token,
                None => return None,
            };

            let res = db
                .select::<Option<SessionSelect>>(("sessions", hashed_token))
                .await
                .ok()?;

            if res.is_some() {
                return Some(generate_access_token());
            } else {
                return None;
            }
        }

        Err(e) => {
            println!("Received invalid message: {e:?}");
            return None;
        }
    };
}

fn generate_access_token() -> AccessToken {
    let mut bytes = [0u8; 48];
    rand::fill(&mut bytes);
    AccessToken(hex::encode(bytes))
}

fn hash_token(token: &str) -> Option<String> {
    let binding = Into::<[u8; 64]>::into(Sha512::digest(token.as_bytes()));
    let res = match str::from_utf8(&binding) {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Server: failed to hash token: {e:?}");
            return None;
        }
    };
    Some(res.to_string())
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
                                Ok(CommandType::DisplayChannels) => {}

                                Ok(CommandType::OpenChannel) => {
                                    send_msg_to_client("Which: ", client);
                                    // let which = client.recv_message()?;
                                    // open_channel(which, db);
                                }

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
    send_msg_to_client(&"Name of channel: ".to_string(), client);

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
        .insert::<Vec<ChannelInsert>>("channels")
        .content(ChannelInsert {
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
    send_msg_to_client(
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
            send_msg_to_client(&"Invalid method, aborting.".to_string(), client);
            return None;
        }
    }
}

async fn log_in(
    db: &Surreal<Any>,
    client: &mut websocket::client::sync::Client<TlsStream<TcpStream>>,
) -> Option<String> {
    let [username, pswd] = get_input_from_user(client, ["Username: ", "Password: "]).ok()?;

    let (OwnedMessage::Text(username), OwnedMessage::Text(pswd)) = (username, pswd) else {
        return None;
    };

    // println!("{username} {pswd}");
    // println!("getting user by username");

    let user = get_user::<UserInsert>(&username, db).await;

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
        eprintln!("Server: failed to fetch user");
        None
    }
}

async fn register(
    db: &Surreal<Any>,
    client: &mut websocket::client::sync::Client<TlsStream<TcpStream>>,
) -> Option<String> {
    let [username, pswd, repeat_pswd] =
        get_input_from_user(client, ["Username: ", "Password: ", "Repeat password: "]).ok()?;

    let (OwnedMessage::Text(username), OwnedMessage::Text(pswd), OwnedMessage::Text(repeat_pswd)) =
        (username, pswd, repeat_pswd)
    else {
        return None;
    };

    if pswd.len() != repeat_pswd.len() || pswd != repeat_pswd {
        send_msg_to_client(&"Entered passwords do not match".to_string(), client);
        return None;
    }

    let user = get_user::<UserSelect>(&username, db).await;

    match user {
        Some(_) => {
            let _ = send_msg_to_client(&"User '{username}' already exists".to_string(), client);
            return None;
        }
        None => {}
    };

    send_msg_to_client(&"Enter phone number (optional): ".to_string(), client);
    let phone_number = match client.recv_message().ok()? {
        OwnedMessage::Text(text) => Some(text),
        _ => None,
    };

    send_msg_to_client(&"Enter email address (optional): ".to_string(), client);
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

async fn display_channels(db: &Surreal<Any>, username: &str) -> Option<Vec<ChannelSelect>> {
    match get_user::<UserSelect>(username, db).await {
        Some(user) => {
            let mut res = db
                .query(
                    "
                        SELECT * FROM channels
                        WHERE members CONTAINS $user;
                    ",
                )
                .bind(("user", user.id))
                .await
                .ok()?;
            let channels: Vec<ChannelSelect> = res.take(0).ok()?;
            return Some(channels);
        }
        None => None,
    }
}

async fn open_channel(which: RecordId, db: &Surreal<Any>) -> Result<Vec<MessageType>> {
    let mut msgs = db
        .query(
            "
                SELECT * FROM messages
                WHERE channel = $channel
            ",
        )
        .bind(("channel", which))
        .await?;
    let res = msgs.take::<Vec<MessageType>>(0)?;
    return Ok(res);
}

async fn get_user<T>(username: &str, db: &Surreal<Any>) -> Option<T>
where
    T: SurrealValue,
{
    let user = db
        .query(
            "
                SELECT * FROM users
                WHERE username = $username
            ",
        )
        .bind(("username", username));

    let user = user.await.ok()?.take::<Option<T>>(0).ok()?;
    return user;
}

fn send_msg_to_client(
    msg: &str,
    client: &mut websocket::client::sync::Client<TlsStream<TcpStream>>,
) {
    let _ = client.send_message(&Message::text(msg));
}

fn get_input_from_user<const N: usize>(
    client: &mut websocket::client::sync::Client<TlsStream<TcpStream>>,
    input_prompts: [&str; N],
) -> Result<[OwnedMessage; N]> {
    let mut ret: Vec<OwnedMessage> = Vec::with_capacity(N);

    for i in 0..input_prompts.len() {
        send_msg_to_client(input_prompts[i], client);
        ret.push(client.recv_message()?);
    }

    Ok(ret
        .try_into()
        .map_err(|_| anyhow!("Error: could not convert to array"))
        .unwrap())
}
