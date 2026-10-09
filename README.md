# Chat App

A local Rust chat app prototype inspired by Telegram. The client is currently an early Ratatui terminal UI, not a complete chat application. The status below describes the current implementation; end-to-end behavior has not been verified.

## Current client status

### Implemented so far

- The client initializes a Ratatui/crossterm terminal and routes drawing by screen state (`Loading`, `LogIn`, or `Main`).
- It connects to a local SurrealDB at `ws://localhost:8000` and selects the login screen when no last-user file is found.
- If `users/last_logged_in` exists, startup selects the main screen and uses that file's contents as the local database name. This does not validate a login session.
- The login view draws username and password labels with a highlighted field. Pressing Tab switches the highlighted field.
- The main view draws placeholder panes for chats, the current chat, and emojis.
- A TLS WebSocket connection attempt to `wss://127.0.0.1:9090` and basic message logging are present in the client source.

### Incomplete or not working

- The login fields do not accept keyboard input, their values are not rendered, and Enter does not submit credentials. Authentication, registration, and session persistence are not wired into the UI.
- There is no registration screen in the current screen state model.
- The main screen contains placeholder content. It shows the literal username `lol`, has no populated chats or messages, and does not send messages from its UI input state.
- Pressing a key on the main screen reaches an unimplemented handler and panics. Login currently handles only Tab, so there is no keyboard quit action.
- WebSocket writes read directly from standard input instead of the TUI. The connection status is also set to Connected before a successful connection is confirmed, and the UI does not display the status.
- The loading screen renderer is empty. Mouse and resize events are received but ignored by the app loop.

## Run locally

Run these commands from the repository root, each in its own terminal:

1. Start SurrealDB: `./bin/start_db.sh`
2. Start the WebSocket server: `./server/run_server.sh`
3. Start the client: `./client/run_client.sh`

The server and client expect local TLS certificate files under `certs/`: the server reads `rustychat.com+4.pem` and `rustychat.com+4-key.pem`, and the client reads `rootCA.pem`. These files are not included in the repository and must be supplied locally for the WebSocket connection.

The run scripts build the workspace before launching their respective binaries.

## Project direction

The project is a hobby prototype. Planned work includes completing the terminal client, improving security, and possibly adding a Tauri GUI later.
