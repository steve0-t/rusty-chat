use surrealdb::{Surreal, engine::any::Any};

use crate::{RecordId, UserSelect, display_channels, get_user, open_channel};

#[tokio::test]
async fn failed_user_queries_return_none() {
    let disconnected = Surreal::<Any>::init();
    assert!(
        get_user::<UserSelect>("alice", &disconnected)
            .await
            .is_none()
    );
}

#[tokio::test]
async fn failed_membership_queries_return_none_rather_than_an_empty_success() {
    let disconnected = Surreal::<Any>::init();
    assert!(display_channels(&disconnected, "alice").await.is_none());
}

#[tokio::test]
async fn failed_message_queries_propagate_the_database_error() {
    let disconnected = Surreal::<Any>::init();
    assert!(
        open_channel(RecordId::new("channels", "missing"), &disconnected)
            .await
            .is_err()
    );
}
