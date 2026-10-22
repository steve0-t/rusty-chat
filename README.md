# Chat App

This README was updated with AI assistance on 2026-10-22.

A Rust chat prototype inspired by Telegram, with a Ratatui terminal client,
a TLS WebSocket server, and SurrealDB storage. Authentication and message delivery
are still incomplete; this is not yet a working end-to-end chat application.

## Current implementation

| Area | Available today | Still unfinished |
| --- | --- | --- |
| Login UI | Username/password entry, Backspace, Tab focus, Space to reveal/hide the password, empty-field validation, Enter emits a login command | The app loop does not execute the login command; validation messages are not rendered; Unicode limits and overflow rendering need fixes |
| Navigation | Loading, login, and main screen states; Esc and Ctrl+C quit | Loading feedback and registration screen; most main-screen keys reach `todo!()` and panic |
| Chat UI | Placeholder chat list, conversation, and emoji panes | Real identity (currently the literal `lol`), channel selection, draft editing/rendering, sending and receiving messages |
| Networking | Local TLS WebSocket connection attempt, stdin-based writes, incoming frame logging, failed-connect notification | TUI integration, accurate connection status, reconnect, heartbeat handling, orderly worker shutdown |
| Accounts and sessions | User/session models, random token generation, token transport helper, saved-user file reader | Registration, password verification, authenticated session restoration, token hashing/storage fixes, expiry/revocation checks, credential persistence |
| Database | Schema for users, channels, messages, sessions; user lookup, membership lookup, channel-history query | Username uniqueness, owner discovery, chronological history, active-session schema support, authorization and complete command dispatch |

Startup connects to `ws://localhost:8000`. If `users/last_logged_in` exists and is
readable as UTF-8, its contents select a database under `chat_app_local` and the
main screen opens. Otherwise the login screen opens. This file is only a local
user hint: it does not authenticate a session, and empty/invalid identities still
need validation. The client currently marks itself Connected before the WebSocket
handshake succeeds.

## Development setup

The workspace contains `client` and `server`. The root binary is a placeholder;
use the package scripts or `cargo run -p client` / `cargo run -p server`.

You need Rust/Cargo with edition 2024 support, OpenSSL development libraries,
`pkg-config`, and SurrealDB to run the application. CI uses Rust 1.95.0 and
SurrealDB 3.2.0, matching the SDK resolved in `Cargo.lock`. On Nix, `nix develop`
provides the tools listed in [flake.nix](flake.nix), including `mkcert`.

Run the following from the repository root:

1. Start the local database in a separate terminal:

   ```sh
   ./bin/start_db.sh
   ```

   This stores data under `data/chat_app_local` and uses the development
   credentials `chat_app_owner` / `security`, which are also hardcoded in the
   server.

2. Import the server schema into that local development database:

   ```sh
   surreal import --endpoint http://localhost:8000 \
     --username chat_app_owner --password security \
     --namespace chat_app --database production db_schemas/schema.surql
   ```

   `production` is the database name currently selected by the server. The
   scripts do not import this schema automatically.

3. Supply the local TLS files (not committed):

   | File | Used by |
   | --- | --- |
   | `certs/rustychat.com+4.pem` | Server certificate, valid for `127.0.0.1` |
   | `certs/rustychat.com+4-key.pem` | Matching server private key in PKCS#8 PEM format |
   | `certs/rootCA.pem` | Client trust root for the server certificate |

4. Launch the server and client in separate terminals:

   ```sh
   ./server/run_server.sh
   ./client/run_client.sh
   ```

   Each script builds the workspace before launching its binary. The source
   targets `wss://127.0.0.1:9090`. Startup and interactive end-to-end operation
   remain unverified; the incomplete authentication and chat paths above still
   apply even with the database and certificates supplied.

## Tests

```sh
./bin/test.sh                 # client and server default suites
./bin/test.sh client
./bin/test.sh server
./bin/test.sh all --offline   # use cached Cargo dependencies
```

The runner executes both selected packages even if one fails, saves logs under
`target/test-results/`, and returns a nonzero exit code if any test fails.
Equivalent direct Cargo invocation:

```sh
cargo test --locked -p client -p server --all-targets --no-fail-fast
```

Default tests use headless Ratatui rendering, temporary files, Tokio channels,
and loopback mock WebSockets. They require no running application/database, TLS
certificates, real terminal, or desktop secret service. Socket tests need
permission to bind loopback ports.

**The suites deliberately include failing contracts for unfinished features.**
These tests compile and execute real application helpers; they are neither
ignored nor marked `should_panic`. They currently keep CI red as a visible
implementation backlog. See the [client coverage notes](client/tests/README.md),
[server coverage notes](server/tests/README.md), and
[proposed acceptance scenarios](docs/acceptance-scenarios.md).

Validated on 2026-10-22 with Rust 1.95.0:

| Suite | Total | Passing | Failing |
| --- | ---: | ---: | ---: |
| Client | 116 | 79 | 37 |
| Server, default | 60 | 47 | 13 |
| Server, live database | 31 | 26 | 5 |

No tests were ignored. The live database run used the installed SurrealDB
3.3 nightly in an isolated in-memory service; CI pins 3.2.0.

Database tests are opt-in. Start a disposable instance, then run the suite from
another terminal:

```sh
surreal start --bind 127.0.0.1:18000 --user root --pass root memory
SURREAL_TEST_ENDPOINT=ws://127.0.0.1:18000 ./bin/test.sh database
```

`SURREAL_TEST_USERNAME` and `SURREAL_TEST_PASSWORD` default to `root`. Each case
imports the table definitions into its own namespace and removes that namespace
afterward, including after assertion failures. The default `all` command does
not enable live database tests.

[GitHub Actions](.github/workflows/tests.yml) runs client, server, and database
suites as independent jobs and uploads their logs even on failure.

## Likely next work

The [acceptance scenarios](docs/acceptance-scenarios.md) propose a progression:
finish login/registration and session handling, make the composer and connection
lifecycle usable, then add authorized channel operations and reliable message
delivery. Membership changes, renaming, and message edits/deletion already have
read-side database contracts. Full command workflows still need APIs and tests.
Pagination, unread counts, and a possible Tauri GUI are later possibilities.
