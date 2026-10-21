use super::*;
use tokio::net::TcpListener;

type ClientSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;
type ServerSocket = WebSocketStream<TcpStream>;

const NETWORK_TIMEOUT: Duration = Duration::from_secs(2);

async fn websocket_pair() -> (ClientSocket, ServerSocket) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind isolated loopback WebSocket fixture");
    let address = format!("ws://{}", listener.local_addr().unwrap());
    let (client, server) = timeout(NETWORK_TIMEOUT, async {
        tokio::join!(tokio_tungstenite::connect_async(address), async {
            let (socket, _) = listener.accept().await.expect("accept mock connection");
            tokio_tungstenite::accept_async(socket).await
        })
    })
    .await
    .expect("mock WebSocket handshake timed out");
    (
        client.expect("connect to mock WebSocket").0,
        server.expect("complete mock WebSocket handshake"),
    )
}

#[test]
fn empty_network_queue_preserves_connection_status() {
    let (_sender, mut receiver) = channel(5);
    for status in [
        ConnectionStatus::Connected,
        ConnectionStatus::Connecting,
        ConnectionStatus::Disconnected,
    ] {
        let mut app = app::App::new();
        app.connection = status;
        let before = format!("{:?}", app.connection);
        poll_networking_messages(&mut receiver, &mut app);
        assert_eq!(format!("{:?}", app.connection), before);
    }
}

#[test]
fn failed_connection_message_marks_every_prior_status_disconnected() {
    for status in [
        ConnectionStatus::Connected,
        ConnectionStatus::Connecting,
        ConnectionStatus::Disconnected,
    ] {
        let (sender, mut receiver) = channel(5);
        sender
            .try_send(NetworkingMessages::FailedToConnect)
            .unwrap();
        let mut app = app::App::new();
        app.connection = status;
        poll_networking_messages(&mut receiver, &mut app);
        assert!(matches!(app.connection, ConnectionStatus::Disconnected));
        assert!(receiver.is_empty());
    }
}

#[test]
fn connection_failure_does_not_erase_login_state() {
    let (sender, mut receiver) = channel(5);
    sender
        .try_send(NetworkingMessages::FailedToConnect)
        .unwrap();
    let mut app = app::App::new();
    app.screen = app::Screen::LogIn(LoginState {
        username: "alice".into(),
        password: "secret".into(),
        focused_field: LoginField::Password,
        show_password: true,
        message: Some("try again".into()),
    });
    let before = format!("{:?}", app.screen);
    poll_networking_messages(&mut receiver, &mut app);
    assert_eq!(format!("{:?}", app.screen), before);
    assert!(!app.should_quit);
}

#[test]
fn connection_failure_does_not_clear_quit_request() {
    let (sender, mut receiver) = channel(5);
    sender
        .try_send(NetworkingMessages::FailedToConnect)
        .unwrap();
    let mut app = app::App::new();
    app.quit();
    poll_networking_messages(&mut receiver, &mut app);
    assert!(app.should_quit);
}

#[test]
fn repeated_failure_messages_are_safe_to_poll_until_empty() {
    let (sender, mut receiver) = channel(5);
    for _ in 0..5 {
        sender
            .try_send(NetworkingMessages::FailedToConnect)
            .unwrap();
    }
    let mut app = app::App::new();
    for _ in 0..8 {
        poll_networking_messages(&mut receiver, &mut app);
        assert!(matches!(app.connection, ConnectionStatus::Disconnected));
    }
    assert!(receiver.is_empty());
}

#[test]
fn queued_failure_is_processed_after_sender_is_dropped() {
    let (sender, mut receiver) = channel(5);
    sender
        .try_send(NetworkingMessages::FailedToConnect)
        .unwrap();
    drop(sender);
    let mut app = app::App::new();
    app.connection = ConnectionStatus::Connected;
    poll_networking_messages(&mut receiver, &mut app);
    assert!(matches!(app.connection, ConnectionStatus::Disconnected));
}

#[test]
fn regression_network_worker_shutdown_marks_connection_disconnected() {
    let (sender, mut receiver) = channel::<NetworkingMessages>(5);
    drop(sender);
    let mut app = app::App::new();
    app.connection = ConnectionStatus::Connected;
    poll_networking_messages(&mut receiver, &mut app);
    assert!(matches!(app.connection, ConnectionStatus::Disconnected));
}

async fn assert_token_frame(token: &str) {
    let (client, mut server) = websocket_pair().await;
    let (mut write, mut read) = client.split();
    timeout(
        NETWORK_TIMEOUT,
        restore_session(token.as_bytes().to_vec(), &mut write, &mut read),
    )
    .await
    .expect("session write timed out")
    .expect("valid UTF-8 session token should be sent");
    let frame = timeout(NETWORK_TIMEOUT, server.next())
        .await
        .expect("mock server read timed out")
        .expect("client closed before sending session token")
        .expect("read session token frame");
    assert!(frame.is_text(), "session token must use a text frame");
    assert_eq!(frame.into_text().unwrap().as_str(), token);
}

#[tokio::test]
async fn restore_session_sends_ascii_token_as_text() {
    assert_token_frame("session-token-123").await;
}

#[tokio::test]
async fn restore_session_preserves_unicode_token_bytes() {
    assert_token_frame("Žofia-密碼-🦀").await;
}

#[tokio::test]
async fn restore_session_preserves_transport_delimiters() {
    assert_token_frame("first\nsecond\r\nthird\0").await;
}

#[tokio::test]
async fn restore_session_preserves_long_token_without_truncation() {
    assert_token_frame(&"token".repeat(4096)).await;
}

#[tokio::test]
async fn restore_session_rejects_invalid_utf8_before_sending_a_frame() {
    let (client, mut server) = websocket_pair().await;
    let (mut write, mut read) = client.split();
    let result = timeout(
        NETWORK_TIMEOUT,
        restore_session(vec![0xff, 0xfe], &mut write, &mut read),
    )
    .await
    .expect("invalid token validation timed out");
    assert!(result.is_err());
    assert!(
        timeout(Duration::from_millis(30), server.next())
            .await
            .is_err(),
        "invalid UTF-8 must not produce a frame"
    );
}

#[tokio::test]
async fn restore_session_propagates_a_closed_websocket_error() {
    let (client, _server) = websocket_pair().await;
    let (mut write, mut read) = client.split();
    timeout(NETWORK_TIMEOUT, write.close())
        .await
        .unwrap()
        .unwrap();
    let result = timeout(
        NETWORK_TIMEOUT,
        restore_session(b"token".to_vec(), &mut write, &mut read),
    )
    .await
    .expect("closed WebSocket write timed out");
    assert!(result.is_err());
}
