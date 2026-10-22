# Server tests

The initial test suite was generated with AI assistance on 2026-10-21.
The test extensions and documentation updates were generated with AI assistance
on 2026-10-22.

Run from the repository root:

```sh
cargo test --locked -p server --bin server
```

Or, from `server/`, run `cargo test --locked`. The default suite requires no
certificates, desktop keyring, running server, or database. Test modules live in
this directory and are included by the binary so they can exercise its private
functions. These modules live under `tests/unit/`, leaving Cargo's normal
integration-test discovery available for future standalone test crates.

Coverage includes every command number and invalid boundaries; token length,
encoding, uniqueness, concurrent generation, secret redaction, independent
SHA-512 vectors and exact byte semantics; all six database model conversions,
required fields, optional fields, native record/datetime values, wrong types,
mixed arrays, Unicode, password projection and Debug redaction; active sessions
with omitted or `NONE` revocation timestamps; and database error propagation
through the existing user, channel and message helpers.

Assertions describe the intended contract and fail normally where implementation
is incomplete. Current known failures include command numbers 3 and 4, SHA-512
storage encoding (raw digest bytes are currently interpreted as UTF-8), bearer
token/password-hash disclosure through `Debug`, and the required revocation
timestamp preventing active session records from being decoded. Fixes should
make these tests pass;
they are not marked ignored or expected to panic.

Validated on 2026-10-22 with Rust 1.95.0: **60 default tests, 47 passing and 13
failing**, with no ignored cases. The two added contracts (password-hash redaction
and absent revocation timestamps) both fail today. The root runner writes output to
`target/test-results/server.log`.

## Live database contracts

Use a disposable SurrealDB 3.2.0 server, matching `Cargo.lock`. For example, in a
separate terminal:

```sh
surreal start --bind 127.0.0.1:8000 --user root --pass root memory
```

Then run:

```sh
SURREAL_TEST_ENDPOINT=ws://127.0.0.1:8000 \
SURREAL_TEST_USERNAME=root \
SURREAL_TEST_PASSWORD=root \
cargo test --locked -p server --features db-tests --bin server -- tests::database:: --test-threads=1
```

The environment variables default to the values shown above. The selected
account needs permission to create and remove test namespaces. Missing or broken
database connectivity fails these tests explicitly. The database feature adds no
embedded storage dependency.

Each test installs the repository schema in a unique
`server_test_<pid>_<random>` namespace and a `contracts` database. The setup strips
the schema's namespace/database definitions and `USE` directives; it does not
import into `chat_app/production`. Cleanup removes only that generated namespace,
including after assertion failures. Interrupting the test process can leave a
test namespace behind, so use an ephemeral database service as the CI job does.

These tests exercise user lookup, literal SQL parameter binding, private channel
membership, owner visibility, record identity, channel history, chronological
ordering, schema defaults and type constraints.

`unit/database_upcoming.rs` adds nine cases for membership changes, owner/member
deduplication, user/channel renames, preserving distinct messages with identical
content, reflecting message edits/deletion in history, and rejecting username
collisions during rename. It is a child of the database module, so the existing
`tests::database::` filter and CI job include it automatically. Eight cases pass;
rename uniqueness fails. Run only the additions against your disposable DB with:

```sh
SURREAL_TEST_ENDPOINT=ws://127.0.0.1:18000 \
cargo test --locked -p server --features db-tests --bin server -- tests::database::upcoming::
```

SQL mutations in these tests arrange database fixtures. Assertions exercise the
existing application read helpers; passing cases do not imply that authorized
join/leave, rename, edit, or delete commands exist. The
[acceptance scenarios](../../docs/acceptance-scenarios.md) specify those future
workflows and the APIs needed to automate them.

Validated database suite: **31 tests, 26 passing and 5 failing**, with no
ignored cases. Local validation used the installed SurrealDB 3.3 nightly in an
isolated in-memory service; CI pins SurrealDB 3.2.0. The five local failures cover
owner visibility, history ordering, username uniqueness on create and rename,
and active sessions without a revocation timestamp. The root runner records output in
`target/test-results/database.log`.

Full authentication, registration, session restoration and TLS/WebSocket
transport remain incomplete in production and need a later transport seam before
the default suite can test those workflows end to end.
