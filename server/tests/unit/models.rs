//! Initially generated with AI assistance on 2026-10-21.

use surrealdb_types::Object;

use crate::{
    ChannelInsert, ChannelSelect, Datetime, MessageType, RecordId, SessionSelect, SurrealValue,
    UserInsert, UserSelect, Value,
};

pub(super) fn timestamp() -> Datetime {
    "2026-10-21T10:20:30.123456Z".parse().unwrap()
}

pub(super) fn user_insert(username: &str) -> UserInsert {
    UserInsert {
        username: username.into(),
        password: "$argon2id$stored-password-hash".into(),
        created_at: timestamp(),
        phone_number: Some("+421 900 123 456".into()),
        email_addr: Some("alice@example.test".into()),
    }
}

fn user_select() -> UserSelect {
    UserSelect {
        id: RecordId::new("users", "alice"),
        username: "alice".into(),
        created_at: timestamp(),
        phone_number: Some("+421 900 123 456".into()),
        email_addr: Some("alice@example.test".into()),
    }
}

pub(super) fn channel_insert(owner: Option<RecordId>, members: Vec<RecordId>) -> ChannelInsert {
    ChannelInsert {
        channel_name: "Engineering 🦀".into(),
        owner,
        members,
        created_at: timestamp(),
    }
}

fn channel_select() -> ChannelSelect {
    ChannelSelect {
        id: RecordId::new("channels", "engineering"),
        channel_name: "Engineering 🦀".into(),
        owner: Some(RecordId::new("users", "alice")),
        members: vec![
            RecordId::new("users", "alice"),
            RecordId::new("users", "bob"),
        ],
        created_at: timestamp(),
    }
}

pub(super) fn message(sender: RecordId, channel: RecordId) -> MessageType {
    MessageType {
        sender,
        channel,
        sent_at: timestamp(),
        status: "delivered".into(),
        content: vec!["hello\nworld".into(), "žluťoučký 🦀\0".into()],
    }
}

fn session() -> SessionSelect {
    SessionSelect {
        user_id: RecordId::new("users", "alice"),
        token_hash: "a".repeat(128),
        device_info: "desktop; Linux".into(),
        created_at: "2026-10-21T10:00:00Z".parse().unwrap(),
        expires_at: "2026-10-22T10:00:00Z".parse().unwrap(),
        last_used_at: "2026-10-21T12:00:00Z".parse().unwrap(),
        revoked_at: "2026-10-21T13:00:00Z".parse().unwrap(),
    }
}

fn as_object(value: Value) -> Object {
    let Value::Object(object) = value else {
        panic!("database model must encode as an object");
    };
    object
}

fn replace(value: Value, field: &str, replacement: Value) -> Value {
    let mut object = as_object(value);
    assert!(object.contains_key(field), "unknown fixture field: {field}");
    object.insert(field, replacement);
    Value::Object(object)
}

fn assert_round_trip<T: SurrealValue>(model: T) {
    let encoded = model.into_value();
    let decoded = T::from_value(encoded.clone()).expect("valid database model");
    assert_eq!(decoded.into_value(), encoded);
}

fn assert_required_fields<T: SurrealValue>(model: T, fields: &[&str]) {
    let object = as_object(model.into_value());
    for &field in fields {
        let mut incomplete = object.clone();
        assert!(incomplete.remove(field).is_some());
        assert!(
            T::from_value(Value::Object(incomplete)).is_err(),
            "missing required {field}"
        );
    }
}

fn assert_invalid_types<T: SurrealValue>(model: T, fields: &[&str]) {
    let value = model.into_value();
    for &field in fields {
        assert!(
            T::from_value(replace(value.clone(), field, Value::Bool(true))).is_err(),
            "invalid type for {field} must be rejected"
        );
    }
}

#[test]
fn user_insert_round_trip_preserves_credentials_and_contact_details() {
    assert_round_trip(user_insert("alice"));
}

#[test]
fn user_select_round_trip_preserves_record_id_and_contact_details() {
    assert_round_trip(user_select());
}

#[test]
fn channel_insert_round_trip_preserves_owner_and_member_order() {
    assert_round_trip(channel_insert(
        Some(RecordId::new("users", "alice")),
        vec![
            RecordId::new("users", "bob"),
            RecordId::new("users", "alice"),
        ],
    ));
}

#[test]
fn channel_select_round_trip_preserves_record_id() {
    assert_round_trip(channel_select());
}

#[test]
fn message_round_trip_preserves_multiline_and_unicode_content() {
    assert_round_trip(message(
        RecordId::new("users", "alice"),
        RecordId::new("channels", "general"),
    ));
}

#[test]
fn session_round_trip_preserves_distinct_lifecycle_timestamps() {
    assert_round_trip(session());
}

#[test]
fn active_sessions_can_be_loaded_without_a_revocation_timestamp() {
    // A session which has never been revoked must not require a fabricated
    // revocation date merely to decode its database record.
    let value = replace(session().into_value(), "revoked_at", Value::None);
    assert!(SessionSelect::from_value(value).is_ok());
}

#[test]
fn usernames_preserve_quotes_unicode_and_sql_metacharacters() {
    for username in [
        "alice' OR true; --",
        "žluťoučký 🦀",
        "line\nbreak",
        "literal\\slash",
    ] {
        let decoded = UserInsert::from_value(user_insert(username).into_value()).unwrap();
        assert_eq!(decoded.username, username);
    }
}

#[test]
fn user_select_never_serializes_a_password() {
    let object = as_object(user_select().into_value());
    assert!(!object.contains_key("password"));
    assert_eq!(object.len(), 5);
}

#[test]
fn user_select_discards_password_from_a_full_database_record() {
    let mut object = as_object(user_select().into_value());
    object.insert("password", "sensitive password hash");
    let selected = UserSelect::from_value(Value::Object(object)).unwrap();
    assert!(!as_object(selected.into_value()).contains_key("password"));
}

#[test]
fn optional_contact_details_support_none() {
    let mut user = user_insert("alice");
    user.phone_number = None;
    user.email_addr = None;
    let value = user.into_value();
    let object = as_object(value.clone());
    assert_eq!(object.get("phone_number"), Some(&Value::None));
    assert_eq!(object.get("email_addr"), Some(&Value::None));
    let decoded = UserInsert::from_value(value).unwrap();
    assert!(decoded.phone_number.is_none());
    assert!(decoded.email_addr.is_none());
}

#[test]
fn optional_contact_details_may_be_omitted_from_database_records() {
    let mut object = as_object(user_select().into_value());
    object.remove("phone_number");
    object.remove("email_addr");
    let selected = UserSelect::from_value(Value::Object(object)).unwrap();
    assert!(selected.phone_number.is_none());
    assert!(selected.email_addr.is_none());
}

#[test]
fn channels_may_have_no_owner_or_members() {
    let decoded = ChannelInsert::from_value(channel_insert(None, vec![]).into_value()).unwrap();
    assert!(decoded.owner.is_none());
    assert!(decoded.members.is_empty());
}

#[test]
fn channel_owner_may_be_omitted_from_database_records() {
    let mut object = as_object(channel_select().into_value());
    object.remove("owner");
    assert!(
        ChannelSelect::from_value(Value::Object(object))
            .unwrap()
            .owner
            .is_none()
    );
}

#[test]
fn native_record_ids_and_datetimes_are_not_flattened_into_strings() {
    let user = as_object(user_select().into_value());
    assert!(matches!(user.get("id"), Some(Value::RecordId(_))));
    assert!(matches!(user.get("created_at"), Some(Value::Datetime(_))));
    let channel = as_object(channel_select().into_value());
    assert!(matches!(channel.get("owner"), Some(Value::RecordId(_))));
    assert!(matches!(channel.get("members"), Some(Value::Array(_))));
}

#[test]
fn user_insert_requires_username_password_and_creation_time() {
    assert_required_fields(
        user_insert("alice"),
        &["username", "password", "created_at"],
    );
}

#[test]
fn user_select_requires_identity_username_and_creation_time() {
    assert_required_fields(user_select(), &["id", "username", "created_at"]);
}

#[test]
fn channel_insert_requires_name_members_and_creation_time() {
    assert_required_fields(
        channel_insert(None, vec![]),
        &["channel_name", "members", "created_at"],
    );
}

#[test]
fn channel_select_requires_identity_name_members_and_creation_time() {
    assert_required_fields(
        channel_select(),
        &["id", "channel_name", "members", "created_at"],
    );
}

#[test]
fn message_requires_all_persisted_fields() {
    assert_required_fields(
        message(
            RecordId::new("users", "alice"),
            RecordId::new("channels", "general"),
        ),
        &["sender", "channel", "sent_at", "status", "content"],
    );
}

#[test]
fn session_requires_identity_token_device_and_timestamps() {
    assert_required_fields(
        session(),
        &[
            "user_id",
            "token_hash",
            "device_info",
            "created_at",
            "expires_at",
            "last_used_at",
        ],
    );
}

#[test]
fn user_insert_rejects_wrong_field_types() {
    assert_invalid_types(
        user_insert("alice"),
        &[
            "username",
            "password",
            "created_at",
            "phone_number",
            "email_addr",
        ],
    );
}

#[test]
fn user_select_rejects_wrong_field_types() {
    assert_invalid_types(
        user_select(),
        &["id", "username", "created_at", "phone_number", "email_addr"],
    );
}

#[test]
fn channel_insert_rejects_wrong_field_types() {
    assert_invalid_types(
        channel_insert(None, vec![]),
        &["channel_name", "owner", "members", "created_at"],
    );
}

#[test]
fn channel_select_rejects_wrong_field_types() {
    assert_invalid_types(
        channel_select(),
        &["id", "channel_name", "owner", "members", "created_at"],
    );
}

#[test]
fn message_rejects_wrong_field_types() {
    assert_invalid_types(
        message(
            RecordId::new("users", "alice"),
            RecordId::new("channels", "general"),
        ),
        &["sender", "channel", "sent_at", "status", "content"],
    );
}

#[test]
fn session_rejects_wrong_field_types() {
    assert_invalid_types(
        session(),
        &[
            "user_id",
            "token_hash",
            "device_info",
            "created_at",
            "expires_at",
            "last_used_at",
            "revoked_at",
        ],
    );
}

#[test]
fn channel_members_accept_valid_string_record_ids() {
    let value = replace(
        channel_select().into_value(),
        "members",
        vec!["users:alice"].into_value(),
    );
    let channel = ChannelSelect::from_value(value).unwrap();
    assert_eq!(channel.members, [RecordId::new("users", "alice")]);
}

#[test]
fn channel_members_reject_malformed_string_record_ids() {
    let value = replace(
        channel_select().into_value(),
        "members",
        vec!["missing-table-and-colon"].into_value(),
    );
    assert!(ChannelSelect::from_value(value).is_err());
}

#[test]
fn channel_member_arrays_reject_mixed_types() {
    let members = vec![
        RecordId::new("users", "alice").into_value(),
        Value::Bool(false),
    ]
    .into_value();
    assert!(
        ChannelSelect::from_value(replace(channel_select().into_value(), "members", members))
            .is_err()
    );
}

#[test]
fn message_content_must_contain_only_strings() {
    let original = message(
        RecordId::new("users", "alice"),
        RecordId::new("channels", "general"),
    )
    .into_value();
    let mixed = vec!["hello".into_value(), Value::Bool(true)].into_value();
    assert!(MessageType::from_value(replace(original, "content", mixed)).is_err());
}

#[test]
fn message_content_can_be_an_empty_array() {
    let mut original = message(
        RecordId::new("users", "alice"),
        RecordId::new("channels", "general"),
    );
    original.content.clear();
    assert!(
        MessageType::from_value(original.into_value())
            .unwrap()
            .content
            .is_empty()
    );
}

#[test]
fn timestamp_fields_reject_unparsed_strings() {
    let value = replace(
        user_insert("alice").into_value(),
        "created_at",
        "2026-10-21T10:20:30Z".into_value(),
    );
    assert!(UserInsert::from_value(value).is_err());
}

#[test]
fn record_id_fields_accept_valid_string_record_ids() {
    let value = replace(user_select().into_value(), "id", "users:alice".into_value());
    assert_eq!(
        UserSelect::from_value(value).unwrap().id,
        RecordId::new("users", "alice")
    );
}

#[test]
fn record_id_fields_reject_malformed_strings() {
    let value = replace(
        user_select().into_value(),
        "id",
        "invalid-record-id".into_value(),
    );
    assert!(UserSelect::from_value(value).is_err());
}

#[test]
fn all_models_reject_non_object_database_values() {
    for invalid in [
        Value::None,
        Value::Null,
        Value::Bool(true),
        "object".into_value(),
        vec![1_i64].into_value(),
    ] {
        assert!(UserInsert::from_value(invalid.clone()).is_err());
        assert!(UserSelect::from_value(invalid.clone()).is_err());
        assert!(ChannelInsert::from_value(invalid.clone()).is_err());
        assert!(ChannelSelect::from_value(invalid.clone()).is_err());
        assert!(MessageType::from_value(invalid.clone()).is_err());
        assert!(SessionSelect::from_value(invalid).is_err());
    }
}
