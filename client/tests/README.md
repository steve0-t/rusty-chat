# Client tests

The initial test suite was generated with AI assistance on 2026-10-21.
The test extensions and documentation updates were generated with AI assistance
on 2026-10-22.

From `client/`, run every client target even when an earlier target fails:

```sh
cargo test --all-targets --no-fail-fast
```

From the repository root, the pipeline wrapper also saves the output in
`target/test-results/client.log`:

```sh
bash bin/test.sh client
```

The suite has 116 tests. Tests live in this workspace; the files under `unit/`
are included by the binary to reach its private networking and file helpers.
The public application modules are available through `client/src/lib.rs`.

| File | Coverage |
| --- | --- |
| `app_state.rs` | Initial state, independent instances, quit, ticks, loading input, global quit shortcuts across screens |
| `login_input.rs` | Field focus, keyboard entry, Unicode, byte capacity, backspace, visibility, validation, login commands, irrelevant navigation |
| `rendering.rs` | Headless Ratatui rendering, labels, shortcuts, masking, focus colors, Unicode, repeated draws, zero/tiny/normal terminal sizes |
| `regressions.rs` | Enabled specifications for unfinished input and rendering behavior |
| `upcoming_features.rs` | Proposed composer behavior, offline draft preservation, navigation, login feedback/privacy, password redraw, draft rendering after resize |
| `unit/networking.rs` | Networking channel events, worker shutdown, draft preservation, WebSocket session-token transport, session requests and errors |
| `unit/session_files.rs` | Session-file reads, missing/invalid/deleted files, Unicode, isolation, empty/whitespace/control-character validation, recovery after corruption |

Tests use Ratatui's `TestBackend`, unique temporary directories, Tokio channels,
and temporary WebSocket servers bound to `127.0.0.1:0`. The network operations
have timeouts. They do not start the application's main loop or require a real
terminal, SurrealDB, TLS certificates, D-Bus, keyring access, or a running chat
server. A sandbox must permit opening loopback sockets for networking tests.
Temporary files are removed after their tests. No process-wide working
directory or environment changes are needed.

## Known failures

Desired-behavior tests remain enabled. They use ordinary assertions and do not
mark panics as success. The expanded run on 2026-10-22 reports **79 passing /
37 failing** tests, with no ignored cases (Rust 1.95.0). These failures expose
the following gaps:

- `regressions.rs`: UTF-8 input can grow past the byte capacity; stale validation
  errors survive successful submission; control/Alt shortcuts and raw control
  characters enter credentials; Ctrl+Shift+C and BackTab are unhandled; main
  screen typing/backspace/navigation panic; errors and drafts are not rendered;
  long usernames disappear; Unicode password masks use byte counts; loading has
  no visible feedback.
- `upcoming_features.rs`: the composer handler panics for input, navigation, and
  submission; drafts are not rendered after resize; editing leaves stale login
  errors; submission leaves revealed passwords visible; Debug exposes passwords.
  The repeated-frame password-hiding contract already passes.
- `unit/networking.rs`: closing the networking channel leaves a Connected or
  Connecting status; `request_session` returns success without sending a request
  and also succeeds on a closed socket. Connection-failure draft preservation
  already passes.
- `unit/session_files.rs`: empty, whitespace-only, and control-containing saved
  identities are accepted as database names. Recovery after replacing a corrupt
  file already passes.

These failures make the pipeline useful as a backlog. Implement the behavior
when ready and keep the assertions. To focus on implemented behavior while
working, run individual targets, for example:

```sh
cargo test --test app_state
cargo test --test login_input
cargo test --test rendering
cargo test --bin client restore_session
```

Run the new UI contracts independently with:

```sh
cargo test --locked -p client --test upcoming_features
```

The extension adds 22 cases: 15 UI contracts, four networking cases, and three
session-file cases. Three pass today and 19 expose unfinished behavior. The
[acceptance scenarios](../../docs/acceptance-scenarios.md) explain the proposed
policies and list further workflows that cannot be exercised until application
APIs exist. Those written scenarios are not counted as automated tests.

The WebSocket tests verify token frame transport and specify session-request
transport behavior without fixing the future request payload format. They do not
claim to verify server authentication, certificate trust, credential storage,
or a completed login flow, which are not wired into the client yet.
