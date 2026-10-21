# Server tests

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
mixed arrays, Unicode and password projection; and database error propagation
through the existing user, channel and message helpers.

Assertions describe the intended contract and fail normally where implementation
is incomplete. Current known failures include command numbers 3 and 4, SHA-512
storage encoding (raw digest bytes are currently interpreted as UTF-8), bearer
token disclosure through `Debug`, and the required revocation timestamp preventing
active session records from being decoded. Fixes should make these tests pass;
they are not marked ignored or expected to panic.

Validated baseline: **58 default tests, 47 passing and 11 failing**, with no
ignored cases. The root runner writes the full output to
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
ordering, schema defaults and type constraints. Desired contracts for unique
usernames, owner discovery, chronological history and active sessions without
revocation timestamps currently expose gaps in the schema/queries and may fail.

Validated database baseline: **22 tests, 18 passing and 4 failing**, with no
ignored cases. Local validation used the installed SurrealDB 3.3 nightly in an
isolated in-memory service; CI pins SurrealDB 3.2.0. The four local failures are
the intended owner visibility, history ordering, username uniqueness and active
session regressions described above. The root runner records output in
`target/test-results/database.log`.

Full authentication, registration, session restoration and TLS/WebSocket
transport remain incomplete in production and need a later transport seam before
the default suite can test those workflows end to end.
