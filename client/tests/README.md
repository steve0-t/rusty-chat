# Client tests

The initial test suite was generated with AI assistance on 2026-10-21.

From `client/`, run every client target even when an earlier target fails:

```sh
cargo test --all-targets --no-fail-fast
```

From the repository root, the pipeline wrapper also saves the output in
`target/test-results/client.log`:

```sh
bash bin/test.sh client
```

The suite has 94 tests. Tests live in this workspace; the files under `unit/`
are included by the binary to reach its private networking and file helpers.
The public application modules are available through `client/src/lib.rs`.

| File | Coverage |
| --- | --- |
| `app_state.rs` | Initial state, independent instances, quit, ticks, loading input, global quit shortcuts across screens |
| `login_input.rs` | Field focus, keyboard entry, Unicode, byte capacity, backspace, visibility, validation, login commands, irrelevant navigation |
| `rendering.rs` | Headless Ratatui rendering, labels, shortcuts, masking, focus colors, Unicode, repeated draws, zero/tiny/normal terminal sizes |
| `regressions.rs` | Enabled specifications for unfinished input and rendering behavior |
| `unit/networking.rs` | Networking channel events, worker shutdown, WebSocket session-token transport and errors |
| `unit/session_files.rs` | Session-file reads, missing/invalid/deleted files, Unicode, isolation, empty-file validation |

Tests use Ratatui's `TestBackend`, unique temporary directories, Tokio channels,
and temporary WebSocket servers bound to `127.0.0.1:0`. The network operations
have timeouts. They do not start the application's main loop or require a real
terminal, SurrealDB, TLS certificates, D-Bus, keyring access, or a running chat
server. A sandbox must permit opening loopback sockets for networking tests.
Temporary files are removed after their tests. No process-wide working
directory or environment changes are needed.

## Known failures

Desired-behavior tests remain enabled. They use ordinary assertions and do not
mark panics as success. The initial full run reports **76 passing / 18 failing**
tests, with no ignored cases. These failures expose the following gaps:

- `regressions.rs`: UTF-8 input can grow past the byte capacity; stale validation
  errors survive successful submission; control/Alt shortcuts and raw control
  characters enter credentials; Ctrl+Shift+C and BackTab are unhandled; main
  screen typing/backspace/navigation panic; errors and drafts are not rendered;
  long usernames disappear; Unicode password masks use byte counts; loading has
  no visible feedback.
- `unit/networking.rs`: closing the networking channel leaves a Connected status.
- `unit/session_files.rs`: an empty last-user file selects an empty database name.

These failures make the pipeline useful as a backlog. Implement the behavior
when ready and keep the assertions. To focus on implemented behavior while
working, run individual targets, for example:

```sh
cargo test --test app_state
cargo test --test login_input
cargo test --test rendering
cargo test --bin client restore_session
```

The WebSocket tests verify the existing token frame transport. They do not
claim to verify server authentication, certificate trust, credential storage,
or a completed login flow, which are not wired into the client yet.
