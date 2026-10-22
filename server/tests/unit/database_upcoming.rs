//! Generated with AI assistance on 2026-10-22.
//!
//! Read-side contracts for future membership management and message mutations.
//! SQL mutations are fixture setup, not implementations of authorized commands.
use super::*;

#[tokio::test]
async fn joining_a_channel_makes_it_visible_on_the_next_lookup() -> Result<()> {
    with_database(|db| async move {
        let alice = create_user(&db, "alice", "alice").await?;
        let channel = create_channel(&db, "joined", None, vec![]).await?;
        assert!(
            crate::display_channels(&db, "alice")
                .await
                .context("before join")?
                .is_empty()
        );
        db.query("UPDATE $channel SET members = [$user];")
            .bind(("channel", channel.id.clone()))
            .bind(("user", alice.id))
            .await?
            .check()?;
        let visible = crate::display_channels(&db, "alice")
            .await
            .context("after join")?;
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].id, channel.id);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn leaving_a_channel_removes_it_from_the_next_lookup() -> Result<()> {
    with_database(|db| async move {
        let alice = create_user(&db, "alice", "alice").await?;
        let bob = create_user(&db, "bob", "bob").await?;
        let channel = create_channel(&db, "left", Some(bob.id), vec![alice.id]).await?;
        assert_eq!(
            crate::display_channels(&db, "alice")
                .await
                .context("before leave")?
                .len(),
            1
        );
        db.query("UPDATE $channel SET members = [];")
            .bind(("channel", channel.id))
            .await?
            .check()?;
        assert!(
            crate::display_channels(&db, "alice")
                .await
                .context("after leave")?
                .is_empty()
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn an_owner_who_is_also_a_member_sees_the_channel_once() -> Result<()> {
    with_database(|db| async move {
        let alice = create_user(&db, "alice", "alice").await?;
        let channel = create_channel(&db, "owned", Some(alice.id.clone()), vec![alice.id]).await?;
        let visible = crate::display_channels(&db, "alice")
            .await
            .context("owner lookup")?;
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].id, channel.id);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn renaming_a_user_preserves_membership_by_record_id() -> Result<()> {
    with_database(|db| async move {
        let alice = create_user(&db, "stable_id", "alice").await?;
        let channel = create_channel(&db, "joined", None, vec![alice.id.clone()]).await?;
        db.query("UPDATE $user SET username = 'Žofia';")
            .bind(("user", alice.id.clone()))
            .await?
            .check()?;
        assert!(crate::get_user::<UserSelect>("alice", &db).await.is_none());
        let renamed = crate::get_user::<UserSelect>("Žofia", &db)
            .await
            .context("renamed user")?;
        assert_eq!(renamed.id, alice.id);
        let visible = crate::display_channels(&db, "Žofia")
            .await
            .context("renamed membership")?;
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].id, channel.id);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn renaming_a_channel_preserves_its_history_and_membership() -> Result<()> {
    with_database(|db| async move {
        let alice = create_user(&db, "alice", "alice").await?;
        let channel = create_channel(&db, "stable_channel", None, vec![alice.id.clone()]).await?;
        let _: Option<MessageType> = db
            .create(("messages", "before_rename"))
            .content(message(alice.id, channel.id.clone()))
            .await?;
        db.query("UPDATE $channel SET channel_name = $name;")
            .bind(("channel", channel.id.clone()))
            .bind(("name", "Renamed 🦀"))
            .await?
            .check()?;
        let visible = crate::display_channels(&db, "alice")
            .await
            .context("renamed channel")?;
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].channel_name, "Renamed 🦀");
        assert_eq!(visible[0].id, channel.id);
        let history = crate::open_channel(channel.id.clone(), &db).await?;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].channel, channel.id);
        assert_eq!(history[0].content, ["hello\nworld", "žluťoučký 🦀\0"]);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn distinct_messages_with_identical_content_are_not_deduplicated() -> Result<()> {
    with_database(|db| async move {
        let alice = create_user(&db, "alice", "alice").await?;
        let channel = create_channel(&db, "history", None, vec![alice.id.clone()]).await?;
        for id in ["first_send", "second_send"] {
            let _: Option<MessageType> = db
                .create(("messages", id))
                .content(message(alice.id.clone(), channel.id.clone()))
                .await?;
        }
        // Separate messages may share text and timestamp. Future retry handling
        // must deduplicate by request/message identity, never by text alone.
        let history = crate::open_channel(channel.id, &db).await?;
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].content, history[1].content);
        assert_eq!(history[0].sent_at, history[1].sent_at);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn editing_message_content_is_reflected_without_changing_sender_or_time() -> Result<()> {
    with_database(|db| async move {
        let alice = create_user(&db, "alice", "alice").await?;
        let channel = create_channel(&db, "history", None, vec![alice.id.clone()]).await?;
        for id in ["edited", "untouched"] {
            let _: Option<MessageType> = db
                .create(("messages", id))
                .content(message(alice.id.clone(), channel.id.clone()))
                .await?;
        }
        db.query("UPDATE messages:edited SET content = $content;")
            .bind(("content", vec!["corrected 🦀\nsecond line"]))
            .await?
            .check()?;
        let history = crate::open_channel(channel.id.clone(), &db).await?;
        assert_eq!(history.len(), 2);
        assert_eq!(
            history
                .iter()
                .filter(|m| m.content == ["corrected 🦀\nsecond line"])
                .count(),
            1
        );
        assert_eq!(
            history
                .iter()
                .filter(|m| m.content == ["hello\nworld", "žluťoučký 🦀\0"])
                .count(),
            1
        );
        for record in history {
            assert_eq!(record.sender, alice.id);
            assert_eq!(record.channel, channel.id);
            assert_eq!(record.sent_at, timestamp());
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn deleting_one_message_keeps_the_remaining_history() -> Result<()> {
    with_database(|db| async move {
        let alice = create_user(&db, "alice", "alice").await?;
        let channel = create_channel(&db, "history", None, vec![alice.id.clone()]).await?;
        for id in ["deleted", "retained"] {
            let mut record = message(alice.id.clone(), channel.id.clone());
            record.content = vec![id.into()];
            let _: Option<MessageType> = db.create(("messages", id)).content(record).await?;
        }
        assert_eq!(crate::open_channel(channel.id.clone(), &db).await?.len(), 2);
        db.query("DELETE messages:deleted;").await?.check()?;
        let history = crate::open_channel(channel.id, &db).await?;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].content, ["retained"]);
        Ok(())
    })
    .await
}

#[tokio::test]
async fn renaming_to_an_existing_username_is_rejected_without_losing_either_user() -> Result<()> {
    with_database(|db| async move {
        let alice = create_user(&db, "alice", "alice").await?;
        let bob = create_user(&db, "bob", "bob").await?;
        let result = db
            .query("UPDATE $user SET username = 'alice';")
            .bind(("user", bob.id.clone()))
            .await?
            .check();
        assert!(
            result.is_err(),
            "username uniqueness must also apply to updates"
        );
        assert_eq!(
            crate::get_user::<UserSelect>("alice", &db)
                .await
                .context("original alice")?
                .id,
            alice.id
        );
        assert_eq!(
            crate::get_user::<UserSelect>("bob", &db)
                .await
                .context("original bob")?
                .id,
            bob.id
        );
        Ok(())
    })
    .await
}
