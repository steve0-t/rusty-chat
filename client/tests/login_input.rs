mod support;

use client::{
    app::{AppCommand, LoginField},
    update::update,
};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use support::{key, login, login_app, login_mut};

#[test]
fn tab_moves_focus_from_username_to_password() {
    let mut app = login_app();
    assert!(update(&mut app, key(KeyCode::Tab)).is_none());
    assert!(matches!(login(&app).focused_field, LoginField::Password));
}

#[test]
fn second_tab_returns_focus_to_username() {
    let mut app = login_app();
    update(&mut app, key(KeyCode::Tab));
    update(&mut app, key(KeyCode::Tab));
    assert!(matches!(login(&app).focused_field, LoginField::Username));
}

#[test]
fn focus_changes_preserve_credentials_visibility_and_validation_message() {
    let mut app = login_app();
    let state = login_mut(&mut app);
    state.username.push_str("alice");
    state.password.push_str("secret");
    state.show_password = true;
    state.message = Some("validation error".into());
    update(&mut app, key(KeyCode::Tab));
    let state = login(&app);
    assert_eq!(state.username, "alice");
    assert_eq!(state.password, "secret");
    assert!(state.show_password);
    assert_eq!(state.message.as_deref(), Some("validation error"));
}

#[test]
fn typing_in_username_leaves_password_unchanged() {
    let mut app = login_app();
    login_mut(&mut app).password.push_str("untouched");
    for ch in "Alice_42!".chars() {
        assert!(update(&mut app, key(KeyCode::Char(ch))).is_none());
    }
    assert_eq!(login(&app).username, "Alice_42!");
    assert_eq!(login(&app).password, "untouched");
}

#[test]
fn typing_in_password_leaves_username_unchanged() {
    let mut app = login_app();
    login_mut(&mut app).username.push_str("alice");
    update(&mut app, key(KeyCode::Tab));
    for ch in "p@ss_W0rd".chars() {
        update(&mut app, key(KeyCode::Char(ch)));
    }
    assert_eq!(login(&app).username, "alice");
    assert_eq!(login(&app).password, "p@ss_W0rd");
}

#[test]
fn unicode_characters_round_trip_in_both_fields() {
    let mut app = login_app();
    let text = "Žofia界🦀";
    for field in 0..2 {
        for ch in text.chars() {
            update(&mut app, key(KeyCode::Char(ch)));
        }
        if field == 0 {
            update(&mut app, key(KeyCode::Tab));
        }
    }
    assert_eq!(login(&app).username, text);
    assert_eq!(login(&app).password, text);
}

#[test]
fn shifted_printable_characters_keep_their_case() {
    let mut app = login_app();
    update(
        &mut app,
        KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT),
    );
    assert_eq!(login(&app).username, "A");
}

#[test]
fn username_stops_accepting_ascii_at_its_capacity() {
    let mut app = login_app();
    let capacity = login(&app).username.capacity();
    for _ in 0..capacity + 8 {
        update(&mut app, key(KeyCode::Char('a')));
    }
    assert_eq!(login(&app).username.len(), capacity);
    assert_eq!(login(&app).username.capacity(), capacity);
}

#[test]
fn password_stops_accepting_ascii_at_its_capacity() {
    let mut app = login_app();
    update(&mut app, key(KeyCode::Tab));
    let capacity = login(&app).password.capacity();
    for _ in 0..capacity + 8 {
        update(&mut app, key(KeyCode::Char('p')));
    }
    assert_eq!(login(&app).password.len(), capacity);
    assert_eq!(login(&app).password.capacity(), capacity);
}

#[test]
fn a_full_username_does_not_block_password_input() {
    let mut app = login_app();
    let capacity = login(&app).username.capacity();
    login_mut(&mut app).username.push_str(&"u".repeat(capacity));
    update(&mut app, key(KeyCode::Tab));
    update(&mut app, key(KeyCode::Char('p')));
    assert_eq!(login(&app).password, "p");
    assert_eq!(login(&app).username.len(), capacity);
}

#[test]
fn backspace_removes_only_the_focused_field() {
    let mut app = login_app();
    login_mut(&mut app).username.push_str("alice");
    login_mut(&mut app).password.push_str("secret");
    update(&mut app, key(KeyCode::Backspace));
    assert_eq!(login(&app).username, "alic");
    assert_eq!(login(&app).password, "secret");
    update(&mut app, key(KeyCode::Tab));
    update(&mut app, key(KeyCode::Backspace));
    assert_eq!(login(&app).username, "alic");
    assert_eq!(login(&app).password, "secre");
}

#[test]
fn backspace_removes_one_unicode_scalar_without_corrupting_utf8() {
    let mut app = login_app();
    login_mut(&mut app).username.push_str("aé界🦀");
    for expected in ["aé界", "aé", "a", ""] {
        update(&mut app, key(KeyCode::Backspace));
        assert_eq!(login(&app).username, expected);
    }
}

#[test]
fn backspace_on_empty_fields_is_a_noop() {
    let mut app = login_app();
    for _ in 0..2 {
        for _ in 0..3 {
            assert!(update(&mut app, key(KeyCode::Backspace)).is_none());
        }
        update(&mut app, key(KeyCode::Tab));
    }
    assert!(login(&app).username.is_empty());
    assert!(login(&app).password.is_empty());
}

#[test]
fn deleting_from_full_field_allows_replacement_character() {
    let mut app = login_app();
    let capacity = login(&app).username.capacity();
    login_mut(&mut app).username.push_str(&"a".repeat(capacity));
    update(&mut app, key(KeyCode::Backspace));
    update(&mut app, key(KeyCode::Char('z')));
    assert_eq!(login(&app).username.len(), capacity);
    assert!(login(&app).username.ends_with('z'));
}

#[test]
fn space_toggles_password_visibility_from_either_field() {
    let mut app = login_app();
    update(&mut app, key(KeyCode::Char(' ')));
    assert!(login(&app).show_password);
    update(&mut app, key(KeyCode::Tab));
    update(&mut app, key(KeyCode::Char(' ')));
    assert!(!login(&app).show_password);
    assert!(login(&app).username.is_empty());
    assert!(login(&app).password.is_empty());
}

#[test]
fn visibility_toggle_does_not_change_credentials() {
    let mut app = login_app();
    login_mut(&mut app).username.push_str("alice");
    login_mut(&mut app).password.push_str("secret");
    for _ in 0..20 {
        update(&mut app, key(KeyCode::Char(' ')));
    }
    assert_eq!(login(&app).username, "alice");
    assert_eq!(login(&app).password, "secret");
    assert!(!login(&app).show_password);
}

#[test]
fn submitting_empty_credentials_sets_helpful_message() {
    let mut app = login_app();
    assert!(update(&mut app, key(KeyCode::Enter)).is_none());
    assert_eq!(
        login(&app).message.as_deref(),
        Some("You haven't entered anything!")
    );
    assert!(!app.should_quit);
}

#[test]
fn submitting_without_password_sets_helpful_message() {
    let mut app = login_app();
    login_mut(&mut app).username.push_str("alice");
    assert!(update(&mut app, key(KeyCode::Enter)).is_none());
    assert_eq!(
        login(&app).message.as_deref(),
        Some("You need to enter password as well!")
    );
}

#[test]
fn submitting_without_username_sets_helpful_message() {
    let mut app = login_app();
    login_mut(&mut app).password.push_str("secret");
    assert!(update(&mut app, key(KeyCode::Enter)).is_none());
    assert_eq!(
        login(&app).message.as_deref(),
        Some("No username was given!")
    );
}

#[test]
fn submitting_complete_credentials_emits_login_command_from_either_field() {
    for focus in [LoginField::Username, LoginField::Password] {
        let mut app = login_app();
        let state = login_mut(&mut app);
        state.username.push_str("alice");
        state.password.push_str("secret");
        state.focused_field = focus;
        assert!(matches!(
            update(&mut app, key(KeyCode::Enter)),
            Some(AppCommand::LogIn)
        ));
        assert!(!app.should_quit);
    }
}

#[test]
fn successful_submit_preserves_credentials_and_selected_field() {
    let mut app = login_app();
    let state = login_mut(&mut app);
    state.username.push_str("alice");
    state.password.push_str("secret");
    state.focused_field = LoginField::Password;
    state.show_password = true;
    assert!(matches!(
        update(&mut app, key(KeyCode::Enter)),
        Some(AppCommand::LogIn)
    ));
    let state = login(&app);
    assert_eq!(state.username, "alice");
    assert_eq!(state.password, "secret");
    assert!(matches!(state.focused_field, LoginField::Password));
    assert!(state.show_password);
}

#[test]
fn unicode_credentials_can_be_submitted() {
    let mut app = login_app();
    login_mut(&mut app).username.push_str("Žofia");
    login_mut(&mut app).password.push_str("密碼🦀");
    assert!(matches!(
        update(&mut app, key(KeyCode::Enter)),
        Some(AppCommand::LogIn)
    ));
}

#[test]
fn repeated_invalid_submit_keeps_same_validation_message() {
    let mut app = login_app();
    for _ in 0..3 {
        assert!(update(&mut app, key(KeyCode::Enter)).is_none());
        assert_eq!(
            login(&app).message.as_deref(),
            Some("You haven't entered anything!")
        );
    }
}

#[test]
fn validation_message_updates_as_missing_field_changes() {
    let mut app = login_app();
    update(&mut app, key(KeyCode::Enter));
    login_mut(&mut app).username.push_str("alice");
    update(&mut app, key(KeyCode::Enter));
    assert_eq!(
        login(&app).message.as_deref(),
        Some("You need to enter password as well!")
    );
    login_mut(&mut app).username.clear();
    login_mut(&mut app).password.push_str("secret");
    update(&mut app, key(KeyCode::Enter));
    assert_eq!(
        login(&app).message.as_deref(),
        Some("No username was given!")
    );
}

#[test]
fn unrelated_navigation_keys_do_not_mutate_login() {
    let mut app = login_app();
    login_mut(&mut app).username.push_str("alice");
    let before = format!("{:?}", login(&app));
    for code in [
        KeyCode::Left,
        KeyCode::Right,
        KeyCode::Up,
        KeyCode::Down,
        KeyCode::Home,
        KeyCode::End,
        KeyCode::PageUp,
        KeyCode::PageDown,
        KeyCode::F(1),
        KeyCode::Null,
    ] {
        assert!(update(&mut app, key(code)).is_none());
        assert_eq!(format!("{:?}", login(&app)), before);
    }
}
