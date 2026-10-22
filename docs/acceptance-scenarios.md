# Proposed feature acceptance scenarios

Generated with AI assistance on 2026-10-22.

These are candidate next features inferred from the current login UI, auth
stubs, channel commands, session models, and message schema. They are a testable
backlog, not a claim that the features exist or a fixed product roadmap.

## Executable contracts added now

The following assertions run against existing production entry points. Missing
behavior fails normally, following the repository's existing test convention.
No placeholder test succeeds merely because an implementation is absent.

| Candidate work | Executable coverage | Current result |
| --- | --- | --- |
| Message composer | `client/tests/upcoming_features.rs`: spaces/Unicode, shifted text, empty backspace, delete/retype, control-character filtering, navigation without losing text | Fails at the main-screen input handler |
| Offline composition | Same file: disconnected/connecting Enter preserves the draft without emitting a command; empty/whitespace drafts are not sent | Fails at the main-screen input handler |
| Draft presentation | Same file: resizing an existing terminal preserves and displays the draft | Draft is not rendered |
| Login feedback/privacy | Same file: editing clears a stale validation error, submission hides a revealed password, Debug redacts passwords, a redraw removes previously visible plaintext | Redraw passes; the other three contracts fail |
| Connection lifecycle | `client/tests/unit/networking.rs`: failure preserves a draft; worker exit while connecting moves to Disconnected | Failure preserves the draft; worker shutdown status fails |
| Session request | Same file: request sends nonempty text and propagates a closed-socket error | Both fail because `request_session` is a no-op |
| Saved user validation | `client/tests/unit/session_files.rs`: reject whitespace-only/control-containing identities; recover after a corrupt file is replaced | Recovery passes; identity validation fails |
| Credential model privacy | `server/tests/unit/models.rs`: Debug redacts stored password hashes; active sessions decode with absent `revoked_at` | Both fail |
| Channel membership/profile changes | `server/tests/unit/database_upcoming.rs`: join/leave visibility, owner/member deduplication, user/channel rename preserving record identity | All pass against existing read helpers |
| Message mutation/history | Same file: edits preserve sender/time, deletion retains other messages, identical text/timestamps do not collapse distinct messages | All pass against existing history helper |
| Account rename uniqueness | Same file: a rename collision is rejected and leaves both users intact | Fails because usernames are not unique |

The database mutation cases use SQL only to arrange fixtures, then call
`display_channels`, `get_user`, or `open_channel`. They do not test authorization,
UI commands, broadcasts, or a completed join/edit/delete feature.

The proposed composer policy allows local editing offline, with no automatic
send on Enter while disconnected or connecting. Drafts containing only whitespace
remain unchanged. The saved-user file is an exact identity: control characters,
including trailing line breaks, invalidate it rather than silently changing the
identity through trimming. Login submission hides the password but preserves its
value for the authentication request. These policies can be revised deliberately
alongside their tests if the product behavior changes.

## Scenarios awaiting application APIs

These are written acceptance cases, **not executable test coverage**. The current
code has no registration state, authentication result events, authorized channel
service, message-send command, or pagination model. Add each scenario to the
automated suite when its entry point exists. Avoid testing a separate fake
implementation that cannot verify the application.

### Accounts and sessions — first priority

| ID | Given / When | Expected result | Needed test entry point |
| --- | --- | --- | --- |
| AUTH-01 | A new username and matching passwords are submitted for registration | One user is created; the stored password is a verifiable salted hash; no plaintext password appears in storage, responses, or logs | Registration service plus credential form |
| AUTH-02 | Registration has a missing required field or mismatched confirmation | A field error is shown, no user/session is created, and retry is possible | Registration validation/command handler |
| AUTH-03 | Two registrations concurrently claim the same username | Exactly one succeeds; the other receives a conflict; existing credentials remain intact | Registration handler and unique database constraint |
| AUTH-04 | Correct credentials are submitted once or Enter is repeated while pending | One in-flight request; success enters Main with the returned identity; failure remains on login with a useful error | Login request/result events and pending state |
| AUTH-05 | An unknown user or wrong password is submitted | No session is issued and both failures expose the same generic authentication error | Authentication service |
| AUTH-06 | A valid saved session token is presented | Its stored hash identifies the session; its user is restored only after server confirmation; `last_used_at` advances | Session lookup/validation with an injectable clock |
| AUTH-07 | A session expires before or exactly at the current time, is revoked, or has an unknown token | Restoration is rejected; no authenticated commands are accepted; the client offers login | Session validator, clock, auth result event |
| AUTH-08 | One of two devices logs out | Its session is revoked and local credentials removed; replay fails; the other device's session remains valid | Logout/revocation service and credential-store interface |
| AUTH-09 | Credential storage is unavailable or a saved token is malformed | Startup remains usable, shows login, and does not claim authentication from `users/last_logged_in` alone | Startup coordinator and injectable credential store |

### Channels and messaging — next priority

| ID | Given / When | Expected result | Needed test entry point |
| --- | --- | --- | --- |
| CHAT-01 | An authenticated user creates a named channel | One channel is created with that owner, appears once in their list, and can be selected | Authorized channel service and UI selection state |
| CHAT-02 | A non-member guesses a private channel ID and requests history or sends a message | Access is denied without returning history or persisting/broadcasting a message | Authenticated actor parameter on history/send handlers |
| CHAT-03 | An authorized membership change adds/removes a user | Listing and subsequent access reflect the change; removed users receive no later channel events | Membership service and subscription authorization |
| CHAT-04 | A member sends Unicode/multiline content in the selected channel | It persists once with the authenticated sender and correct channel; intended recipients receive it; other channels do not | Message command/result events and broadcast transport |
| CHAT-05 | A send fails or the connection drops before acknowledgement | Draft/pending message remains recoverable; UI shows failure/pending rather than delivery; retry is possible | Delivery state and server acknowledgement |
| CHAT-06 | A request is retried with the same request ID, then identical text is sent with a new ID | Retry creates no duplicate; the new intentional message remains distinct | Stable message/request IDs and idempotent persistence |
| CHAT-07 | An author edits/deletes a message, or another ordinary member attempts it | Authorized mutation affects only that message and reaches subscribers; unauthorized mutation leaves storage unchanged | Mutation authorization, message identity, event handling |
| CHAT-08 | The user switches between channels with unsent text | Each channel restores its own draft; sending never uses a different channel's draft | Channel-keyed draft state |

### Connection reliability and later features

| ID | Given / When | Expected result | Needed test entry point |
| --- | --- | --- | --- |
| NET-01 | A WebSocket connection succeeds/fails or reconnect is attempted | Status follows Disconnected → Connecting → Connected only after a successful handshake; failure returns to Disconnected and preserves drafts | Connection result events and injectable connector |
| NET-02 | The server sends Ping, Pong, Close, or an unsupported binary frame | Ping receives the matching Pong; control frames do not enter message history; close/unsupported data are handled without panicking | Transport-independent frame handler |
| NET-03 | A peer closes or the user quits while workers are waiting | Read/write workers finish within a bounded timeout; the terminal is restored; no stale Connected status remains | Cancellation-aware workers and terminal lifecycle interface |
| NET-04 | TLS certificate trust, hostname verification, or handshake fails | Credentials are never sent; a connection error is surfaced; login/drafts remain recoverable | TLS fixture with a test CA and failed-handshake paths |
| LATER-01 | History is fetched in pages while messages share timestamps or new messages arrive | A stable cursor avoids duplicates/skips; loading older history preserves scroll position | Message IDs, deterministic ordering, pagination API |
| LATER-02 | Messages arrive in an inactive channel, then that channel is read | Unread count advances once per message and clears/advances on the read action; replayed events do not inflate it | Message identity and read-receipt/unread state |

Use isolated database namespaces for persistence cases, fake clocks for expiry
boundaries, and bounded loopback fixtures for transport. Keep authentication
tests separate from token-frame tests: successful writing of a token frame does
not demonstrate successful authentication. GUI/attachment/reaction behavior
should get its own scenarios if those features are selected for development.
