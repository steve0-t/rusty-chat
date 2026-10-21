//! Opt-in contracts against a disposable SurrealDB 3.2 server.
//!
//! Each case owns its own random namespace. Production namespace/database
//! directives are deliberately removed before applying the repository schema.

use std::{future::Future, time::Duration};

use anyhow::{Context, Result};
use surrealdb::{Surreal, engine::any::Any, opt::auth::Root};
use tokio::time::timeout;

use super::models::{channel_insert, message, timestamp, user_insert};
use crate::{ChannelSelect, MessageType, RecordId, SurrealValue, UserSelect, Value};

struct Database {
    db: Surreal<Any>,
    namespace: String,
}

impl Database {
    async fn connect() -> Result<Self> {
        let endpoint =
            std::env::var("SURREAL_TEST_ENDPOINT").unwrap_or_else(|_| "ws://127.0.0.1:8000".into());
        let username = std::env::var("SURREAL_TEST_USERNAME").unwrap_or_else(|_| "root".into());
        let password = std::env::var("SURREAL_TEST_PASSWORD").unwrap_or_else(|_| "root".into());
        let db = timeout(
            Duration::from_secs(10),
            surrealdb::engine::any::connect(endpoint),
        )
        .await
        .context("test database connection timed out")??;
        db.signin(Root { username, password }).await?;

        let random: [u8; 16] = rand::random();
        let namespace = format!("server_test_{}_{}", std::process::id(), hex::encode(random));
        db.use_ns(&namespace).use_db("contracts").await?;
        let fixture = Self { db, namespace };
        let schema = isolated_schema();
        let installed = match fixture.db.query(schema).await {
            Ok(response) => response.check().map(|_| ()),
            Err(error) => Err(error),
        };
        if let Err(error) = installed {
            fixture.cleanup().await?;
            return Err(error.into());
        }
        Ok(fixture)
    }

    async fn cleanup(&self) -> Result<()> {
        // The identifier is generated above from a fixed prefix, PID and hex.
        // No user-supplied namespace or production identifier is interpolated.
        self.db
            .query(format!("REMOVE NAMESPACE {};", self.namespace))
            .await?
            .check()?;
        Ok(())
    }
}

fn isolated_schema() -> String {
    include_str!("../../../db_schemas/schema.surql")
        .lines()
        .filter(|line| {
            let line = line.trim().to_ascii_lowercase();
            !(line.starts_with("define namespace ")
                || line.starts_with("define database ")
                || line.starts_with("use ns ")
                || line.starts_with("use db ")
                || line == "option import;")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

async fn with_database<F, Fut>(case: F) -> Result<()>
where
    F: FnOnce(Surreal<Any>) -> Fut + Send + 'static,
    Fut: Future<Output = Result<()>> + Send + 'static,
{
    let fixture = timeout(Duration::from_secs(15), Database::connect())
        .await
        .context("database fixture setup exceeded 15 seconds")??;
    let db = fixture.db.clone();
    // Join the test task before cleanup so assertion failures still remove the
    // isolated namespace. A broken database/service fails this suite explicitly.
    let mut task = tokio::spawn(async move { case(db).await });
    let outcome = timeout(Duration::from_secs(30), &mut task).await;
    if outcome.is_err() {
        task.abort();
        let _ = task.await;
    }
    let cleanup = timeout(Duration::from_secs(10), fixture.cleanup())
        .await
        .context("database fixture cleanup exceeded 10 seconds");
    let result = outcome
        .context("database test exceeded 30 seconds")?
        .context("database test panicked")?;
    cleanup??;
    result
}

async fn create_user(db: &Surreal<Any>, id: &str, username: &str) -> Result<UserSelect> {
    let created: Option<UserSelect> = db
        .create(("users", id))
        .content(user_insert(username))
        .await?;
    created.context("user should be created")
}

async fn create_channel(
    db: &Surreal<Any>,
    id: &str,
    owner: Option<RecordId>,
    members: Vec<RecordId>,
) -> Result<ChannelSelect> {
    let created: Option<ChannelSelect> = db
        .create(("channels", id))
        .content(channel_insert(owner, members))
        .await?;
    created.context("channel should be created")
}

#[test]
fn schema_installation_cannot_switch_to_the_production_namespace_or_database() {
    let schema = isolated_schema().to_ascii_lowercase();
    assert!(!schema.contains("use ns "));
    assert!(!schema.contains("use db "));
    assert!(!schema.contains("define namespace "));
    assert!(!schema.contains("define database "));
    for table in ["users", "channels", "messages", "sessions"] {
        assert!(schema.contains(&format!("define table if not exists {table}")));
    }
}

#[tokio::test]
async fn get_user_returns_the_requested_user() -> Result<()> {
    with_database(|db| async move {
        let inserted = create_user(&db, "alice", "alice").await?;
        let user = crate::get_user::<UserSelect>("alice", &db)
            .await
            .context("existing user")?;
        assert_eq!(user.id, inserted.id);
        assert_eq!(user.username, "alice");
        assert_eq!(user.created_at, timestamp());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn get_user_returns_none_for_unknown_username() -> Result<()> {
    with_database(|db| async move {
        create_user(&db, "alice", "alice").await?;
        assert!(
            crate::get_user::<UserSelect>("missing", &db)
                .await
                .is_none()
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn get_user_uses_exact_case_sensitive_username_matching() -> Result<()> {
    with_database(|db| async move {
        create_user(&db, "alice", "Alice").await?;
        assert!(crate::get_user::<UserSelect>("alice", &db).await.is_none());
        assert!(crate::get_user::<UserSelect>("Alice", &db).await.is_some());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn get_user_binds_sql_metacharacters_as_literal_data() -> Result<()> {
    with_database(|db| async move {
        let username = "alice' OR true; DELETE users; --";
        let inserted = create_user(&db, "quoted", username).await?;
        create_user(&db, "bob", "bob").await?;
        let selected = crate::get_user::<UserSelect>(username, &db)
            .await
            .context("literal username")?;
        assert_eq!(selected.id, inserted.id);
        assert!(crate::get_user::<UserSelect>("bob", &db).await.is_some());
        assert!(
            crate::get_user::<UserSelect>("' OR true", &db)
                .await
                .is_none()
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn get_user_can_decode_the_password_only_when_requested() -> Result<()> {
    with_database(|db| async move {
        create_user(&db, "alice", "alice").await?;
        let user = crate::get_user::<crate::UserInsert>("alice", &db)
            .await
            .context("credential lookup")?;
        assert_eq!(user.password, "$argon2id$stored-password-hash");
        let public = crate::get_user::<UserSelect>("alice", &db)
            .await
            .context("public lookup")?;
        let Value::Object(public) = public.into_value() else {
            panic!("user object")
        };
        assert!(!public.contains_key("password"));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn username_uniqueness_prevents_ambiguous_authentication() -> Result<()> {
    with_database(|db| async move {
        create_user(&db, "first", "alice").await?;
        let duplicate = create_user(&db, "second", "alice").await;
        assert!(
            duplicate.is_err(),
            "a second record with the same username must be rejected"
        );
        assert!(crate::get_user::<UserSelect>("alice", &db).await.is_some());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn display_channels_returns_none_for_unknown_user() -> Result<()> {
    with_database(|db| async move {
        assert!(crate::display_channels(&db, "missing").await.is_none());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn display_channels_returns_an_empty_list_for_a_user_without_memberships() -> Result<()> {
    with_database(|db| async move {
        create_user(&db, "alice", "alice").await?;
        let channels = crate::display_channels(&db, "alice")
            .await
            .context("known user")?;
        assert!(channels.is_empty());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn display_channels_only_returns_channels_containing_the_user_record() -> Result<()> {
    with_database(|db| async move {
        let alice = create_user(&db, "alice", "alice").await?;
        let bob = create_user(&db, "bob", "bob").await?;
        let joined = create_channel(
            &db,
            "joined",
            Some(bob.id.clone()),
            vec![alice.id, bob.id.clone()],
        )
        .await?;
        create_channel(&db, "private", Some(bob.id.clone()), vec![bob.id]).await?;
        create_channel(&db, "empty", None, vec![]).await?;
        let channels = crate::display_channels(&db, "alice")
            .await
            .context("channel lookup")?;
        assert_eq!(channels.len(), 1);
        assert_eq!(channels[0].id, joined.id);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn channel_owner_can_discover_their_channel_without_duplicate_membership() -> Result<()> {
    with_database(|db| async move {
        let alice = create_user(&db, "alice", "alice").await?;
        let owned = create_channel(&db, "owned", Some(alice.id), vec![]).await?;
        let channels = crate::display_channels(&db, "alice")
            .await
            .context("owner channel lookup")?;
        assert!(
            channels.iter().any(|channel| channel.id == owned.id),
            "owners must be able to find channels they created"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn display_channels_matches_members_by_record_id_rather_than_username() -> Result<()> {
    with_database(|db| async move {
        let user = create_user(&db, "internal_id", "visible_name").await?;
        let joined = create_channel(&db, "joined", None, vec![user.id]).await?;
        let channels = crate::display_channels(&db, "visible_name")
            .await
            .context("membership lookup")?;
        assert_eq!(channels.len(), 1);
        assert_eq!(channels[0].id, joined.id);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn open_channel_returns_no_messages_when_empty() -> Result<()> {
    with_database(|db| async move {
        let channel = create_channel(&db, "empty", None, vec![]).await?;
        assert!(crate::open_channel(channel.id, &db).await?.is_empty());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn open_channel_returns_only_messages_for_the_requested_record() -> Result<()> {
    with_database(|db| async move {
        let alice = create_user(&db, "alice", "alice").await?;
        let first =
            create_channel(&db, "first", Some(alice.id.clone()), vec![alice.id.clone()]).await?;
        let second = create_channel(&db, "second", None, vec![alice.id.clone()]).await?;
        for (id, channel) in [
            ("first_message", first.id.clone()),
            ("second_message", second.id),
        ] {
            let _: Option<MessageType> = db
                .create(("messages", id))
                .content(message(alice.id.clone(), channel))
                .await?;
        }
        let messages = crate::open_channel(first.id.clone(), &db).await?;
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].channel, first.id);
        assert_eq!(messages[0].sender, alice.id);
        assert_eq!(messages[0].content, ["hello\nworld", "žluťoučký 🦀\0"]);
        assert_eq!(messages[0].status, "delivered");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn open_channel_returns_messages_in_chronological_order() -> Result<()> {
    with_database(|db| async move {
        let alice = create_user(&db, "alice", "alice").await?;
        let channel = create_channel(&db, "history", None, vec![alice.id.clone()]).await?;
        // Record IDs sort in the opposite order to their timestamps.
        for (id, sent_at) in [
            ("a_newer", "2026-10-21T12:00:00Z"),
            ("z_older", "2026-10-21T11:00:00Z"),
        ] {
            let mut content = message(alice.id.clone(), channel.id.clone());
            content.sent_at = sent_at.parse().unwrap();
            let _: Option<MessageType> = db.create(("messages", id)).content(content).await?;
        }
        let messages = crate::open_channel(channel.id, &db).await?;
        assert_eq!(messages.len(), 2);
        assert!(
            messages[0].sent_at <= messages[1].sent_at,
            "message history must be oldest first"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn schema_defaults_user_creation_time_and_allows_missing_optional_contact_details()
-> Result<()> {
    with_database(|db| async move {
        // Accommodate a nearby test DB with a small clock offset while
        // rejecting an epoch or other static default.
        let earliest = crate::Datetime::now().timestamp() - 5;
        let mut response = db
            .query("CREATE users:defaults SET username = 'defaults', password = 'hash';")
            .await?
            .check()?;
        let users: Vec<UserSelect> = response.take(0)?;
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].username, "defaults");
        assert!(users[0].phone_number.is_none());
        assert!(users[0].email_addr.is_none());
        let latest = crate::Datetime::now().timestamp() + 5;
        assert!((earliest..=latest).contains(&users[0].created_at.timestamp()));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn schema_rejects_users_without_a_password() -> Result<()> {
    with_database(|db| async move {
        assert!(
            db.query("CREATE users:no_password SET username = 'missing';")
                .await?
                .check()
                .is_err()
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn schema_rejects_non_string_usernames() -> Result<()> {
    with_database(|db| async move {
        assert!(
            db.query("CREATE users:wrong_type SET username = 42, password = 'hash';")
                .await?
                .check()
                .is_err()
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn schema_rejects_channel_members_from_the_wrong_table() -> Result<()> {
    with_database(|db| async move {
        create_channel(&db, "other", None, vec![]).await?;
        let result = db
            .query(
                "CREATE channels:invalid SET channel_name = 'wrong', members = [channels:other];",
            )
            .await?
            .check();
        assert!(result.is_err());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn schema_rejects_message_content_with_non_string_elements() -> Result<()> {
    with_database(|db| async move {
        let alice = create_user(&db, "alice", "alice").await?;
        let channel = create_channel(&db, "general", None, vec![alice.id]).await?;
        let result = db.query("CREATE messages:invalid SET sender = users:alice, channel = $channel, status = 'sent', content = ['hello', 42];")
            .bind(("channel", channel.id)).await?.check();
        assert!(result.is_err());
        Ok(())
    }).await
}

#[tokio::test]
async fn schema_rejects_message_channel_references_to_users() -> Result<()> {
    with_database(|db| async move {
        create_user(&db, "alice", "alice").await?;
        let result = db.query("CREATE messages:invalid SET sender = users:alice, channel = users:alice, status = 'sent', content = ['hello'];").await?.check();
        assert!(result.is_err());
        Ok(())
    }).await
}

#[tokio::test]
async fn schema_allows_active_sessions_without_a_revocation_timestamp() -> Result<()> {
    with_database(|db| async move {
        let user = create_user(&db, "alice", "alice").await?;
        let mut response = db
            .query(
                "CREATE sessions:active SET user_id = $user, token_hash = $hash, \
                 device_info = 'test desktop', created_at = $created, \
                 expires_at = $expires, last_used_at = $created;",
            )
            .bind(("user", user.id))
            .bind(("hash", "a".repeat(128)))
            .bind(("created", timestamp()))
            .bind((
                "expires",
                "2026-10-22T10:20:30Z".parse::<crate::Datetime>()?,
            ))
            .await?
            .check()
            .context("an active session must not require a fabricated revocation timestamp")?;
        let sessions: Vec<Value> = response.take(0)?;
        assert_eq!(sessions.len(), 1);
        let Value::Object(session) = &sessions[0] else {
            panic!("session record");
        };
        assert!(
            session
                .get("revoked_at")
                .is_none_or(|value| *value == Value::None)
        );
        Ok(())
    })
    .await
}
