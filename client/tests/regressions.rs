//! Desired behavior for known unfinished paths. These tests intentionally remain
//! enabled so the pipeline reports the gaps until the implementation catches up.
mod support;

use client::{
    app::{App, AppCommand, LoginField, MainState, Screen},
    ui,
    update::update,
};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use support::{buffer_text, key, login, login_app, login_mut, login_state, render};

#[test]
fn username_rejects_a_unicode_character_that_exceeds_byte_capacity() {
    let mut app = login_app();
    let capacity = login(&app).username.capacity();
    login_mut(&mut app)
        .username
        .push_str(&"a".repeat(capacity - 1));
    update(&mut app, key(KeyCode::Char('🦀')));
    assert!(
        login(&app).username.len() <= capacity,
        "input exceeded configured byte capacity"
    );
    assert_eq!(login(&app).username.capacity(), capacity);
}

#[test]
fn password_rejects_a_unicode_character_that_exceeds_byte_capacity() {
    let mut app = login_app();
    login_mut(&mut app).focused_field = LoginField::Password;
    let capacity = login(&app).password.capacity();
    login_mut(&mut app)
        .password
        .push_str(&"p".repeat(capacity - 1));
    update(&mut app, key(KeyCode::Char('🦀')));
    assert!(
        login(&app).password.len() <= capacity,
        "input exceeded configured byte capacity"
    );
    assert_eq!(login(&app).password.capacity(), capacity);
}

#[test]
fn successful_login_submission_clears_previous_validation_error() {
    let mut app = login_app();
    update(&mut app, key(KeyCode::Enter));
    login_mut(&mut app).username.push_str("alice");
    login_mut(&mut app).password.push_str("secret");
    assert!(matches!(
        update(&mut app, key(KeyCode::Enter)),
        Some(AppCommand::LogIn)
    ));
    assert!(
        login(&app).message.is_none(),
        "successful validation left a stale error"
    );
}

#[test]
fn control_shortcuts_do_not_insert_letters_into_credentials() {
    let mut app = login_app();
    for code in [KeyCode::Char('a'), KeyCode::Char('v')] {
        update(&mut app, KeyEvent::new(code, KeyModifiers::CONTROL));
    }
    assert!(login(&app).username.is_empty());
}

#[test]
fn alt_shortcuts_do_not_insert_letters_into_credentials() {
    let mut app = login_app();
    update(
        &mut app,
        KeyEvent::new(KeyCode::Char('x'), KeyModifiers::ALT),
    );
    assert!(login(&app).username.is_empty());
}

#[test]
fn control_shift_c_quits_even_when_shift_modifier_is_present() {
    let mut app = login_app();
    update(
        &mut app,
        KeyEvent::new(
            KeyCode::Char('C'),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        ),
    );
    assert!(app.should_quit);
    assert!(login(&app).username.is_empty());
}

#[test]
fn backtab_cycles_focus_backwards() {
    let mut app = login_app();
    update(&mut app, key(KeyCode::BackTab));
    assert!(matches!(login(&app).focused_field, LoginField::Password));
    update(&mut app, key(KeyCode::BackTab));
    assert!(matches!(login(&app).focused_field, LoginField::Username));
}

#[test]
fn control_characters_cannot_be_inserted_into_credentials() {
    let mut app = login_app();
    for ch in ['\0', '\n', '\r', '\t', '\u{1b}'] {
        update(&mut app, key(KeyCode::Char(ch)));
    }
    assert!(login(&app).username.is_empty());
}

#[test]
fn main_screen_accepts_message_draft_without_panicking() {
    let mut app = App::new();
    app.screen = Screen::Main(MainState {
        input: "hello".into(),
    });
    assert!(update(&mut app, key(KeyCode::Char('!'))).is_none());
    let Screen::Main(state) = app.screen else {
        panic!("screen changed")
    };
    assert_eq!(state.input, "hello!");
}

#[test]
fn main_screen_backspace_removes_a_unicode_character_without_panicking() {
    let mut app = App::new();
    app.screen = Screen::Main(MainState {
        input: "hello🦀".into(),
    });
    update(&mut app, key(KeyCode::Backspace));
    let Screen::Main(state) = app.screen else {
        panic!("screen changed")
    };
    assert_eq!(state.input, "hello");
}

#[test]
fn main_screen_ignores_unhandled_navigation_without_panicking() {
    let mut app = App::new();
    app.screen = Screen::Main(MainState {
        input: "draft".into(),
    });
    assert!(update(&mut app, key(KeyCode::F(12))).is_none());
    let Screen::Main(state) = app.screen else {
        panic!("screen changed")
    };
    assert_eq!(state.input, "draft");
}

#[test]
fn login_validation_error_is_visible_to_user() {
    let mut state = login_state();
    state.message = Some("No username was given!".into());
    let buffer = render(120, 60, |frame| ui::render_login_screen(&state, frame));
    assert!(buffer_text(&buffer).contains("No username was given!"));
}

#[test]
fn long_username_keeps_its_tail_visible() {
    let mut state = login_state();
    state.username = format!("{}TAIL123", "prefix".repeat(20));
    let buffer = render(120, 60, |frame| ui::render_login_screen(&state, frame));
    assert!(
        buffer_text(&buffer).contains("TAIL123"),
        "overflowing username disappeared"
    );
}

#[test]
fn unicode_password_is_masked_once_per_character() {
    let mut state = login_state();
    state.password.push_str("é🦀");
    let buffer = render(120, 60, |frame| ui::render_login_screen(&state, frame));
    let text = buffer_text(&buffer);
    assert_eq!(text.matches('*').count(), state.password.chars().count());
    assert!(!text.contains("é🦀"));
}

#[test]
fn loading_screen_displays_progress_feedback() {
    let buffer = render(80, 24, ui::render_loading_screen);
    assert!(
        !buffer_text(&buffer).trim().is_empty(),
        "loading screen has no progress feedback"
    );
}

#[test]
fn main_screen_displays_the_current_message_draft() {
    let state = MainState {
        input: "unsent draft".into(),
    };
    let buffer = render(120, 60, |frame| ui::render_main_screen(&state, frame));
    assert!(buffer_text(&buffer).contains("unsent draft"));
}
