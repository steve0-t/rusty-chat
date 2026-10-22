//! Generated with AI assistance on 2026-10-22.
//!
//! Executable contracts for the next TUI work. Failures are intentional backlog
//! signals, never expected panics. See docs/acceptance-scenarios.md for scope.
mod support;

use client::{
    app::{App, AppCommand, ConnectionStatus, MainState, Screen},
    ui,
    update::update,
};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend};
use support::{buffer_text, key, login, login_app, login_mut, login_state};

fn composer(draft: &str, connection: ConnectionStatus) -> App {
    let mut app = App::new();
    let mut input = String::with_capacity(256);
    input.push_str(draft);
    app.screen = Screen::Main(MainState { input });
    app.connection = connection;
    app
}

fn draft(app: &App) -> &str {
    let Screen::Main(state) = &app.screen else {
        panic!("editing a draft must keep the main screen open");
    };
    &state.input
}

#[test]
fn composer_accepts_spaces_and_unicode_while_offline() {
    let mut app = composer("", ConnectionStatus::Disconnected);
    for ch in "Ahoj 世界 🦀".chars() {
        assert!(update(&mut app, key(KeyCode::Char(ch))).is_none());
    }
    assert_eq!(draft(&app), "Ahoj 世界 🦀");
    assert!(!app.should_quit);
}

#[test]
fn composer_keeps_shifted_printable_characters() {
    let mut app = composer("", ConnectionStatus::Connected);
    for ch in ['A', '!'] {
        assert!(
            update(
                &mut app,
                KeyEvent::new(KeyCode::Char(ch), KeyModifiers::SHIFT)
            )
            .is_none()
        );
    }
    assert_eq!(draft(&app), "A!");
}

#[test]
fn composer_backspace_on_an_empty_draft_is_safe() {
    let mut app = composer("", ConnectionStatus::Connected);
    assert!(update(&mut app, key(KeyCode::Backspace)).is_none());
    assert_eq!(draft(&app), "");
}

#[test]
fn composer_can_delete_and_replace_an_entire_unicode_draft() {
    let mut app = composer("é🦀", ConnectionStatus::Disconnected);
    for expected in ["é", "", ""] {
        assert!(update(&mut app, key(KeyCode::Backspace)).is_none());
        assert_eq!(draft(&app), expected);
    }
    assert!(update(&mut app, key(KeyCode::Char('界'))).is_none());
    assert_eq!(draft(&app), "界");
}

#[test]
fn composer_rejects_raw_terminal_control_characters() {
    for ch in ['\0', '\n', '\r', '\t', '\u{1b}', '\u{7f}'] {
        let mut app = composer("keep", ConnectionStatus::Connected);
        assert!(update(&mut app, key(KeyCode::Char(ch))).is_none());
        assert_eq!(draft(&app), "keep", "unexpected insertion of {ch:?}");
    }
}

#[test]
fn composer_shortcuts_do_not_insert_literal_letters() {
    for modifiers in [KeyModifiers::CONTROL, KeyModifiers::ALT] {
        let mut app = composer("keep", ConnectionStatus::Connected);
        assert!(update(&mut app, KeyEvent::new(KeyCode::Char('v'), modifiers)).is_none());
        assert_eq!(draft(&app), "keep");
        assert!(!app.should_quit);
    }
}

#[test]
fn offline_enter_preserves_unsent_draft_without_emitting_a_command() {
    let mut app = composer("send later", ConnectionStatus::Disconnected);
    assert!(update(&mut app, key(KeyCode::Enter)).is_none());
    assert_eq!(draft(&app), "send later");
}

#[test]
fn connecting_enter_preserves_unsent_draft_without_emitting_a_command() {
    let mut app = composer("send later", ConnectionStatus::Connecting);
    assert!(update(&mut app, key(KeyCode::Enter)).is_none());
    assert_eq!(draft(&app), "send later");
}

#[test]
fn empty_or_whitespace_only_drafts_do_not_emit_commands() {
    for input in ["", "   ", "\u{2003}"] {
        let mut app = composer(input, ConnectionStatus::Connected);
        assert!(update(&mut app, key(KeyCode::Enter)).is_none());
        assert_eq!(draft(&app), input);
    }
}

#[test]
fn composer_navigation_preserves_draft_text() {
    let mut app = composer("keep this draft", ConnectionStatus::Connected);
    for code in [
        KeyCode::Left,
        KeyCode::Right,
        KeyCode::Home,
        KeyCode::End,
        KeyCode::Tab,
    ] {
        assert!(update(&mut app, key(code)).is_none());
        assert_eq!(draft(&app), "keep this draft");
    }
}

#[test]
fn correcting_a_login_field_dismisses_its_stale_validation_error() {
    let mut app = login_app();
    update(&mut app, key(KeyCode::Enter));
    assert!(login(&app).message.is_some());
    update(&mut app, key(KeyCode::Char('a')));
    assert_eq!(login(&app).username, "a");
    assert!(login(&app).message.is_none());
}

#[test]
fn submitting_a_visible_password_hides_it_again() {
    let mut app = login_app();
    let state = login_mut(&mut app);
    state.username.push_str("alice");
    state.password.push_str("private-password");
    state.show_password = true;
    assert!(matches!(
        update(&mut app, key(KeyCode::Enter)),
        Some(AppCommand::LogIn)
    ));
    assert!(!login(&app).show_password);
    assert_eq!(login(&app).password, "private-password");
}

#[test]
fn debug_output_redacts_passwords_even_when_the_ui_reveals_them() {
    let mut app = login_app();
    login_mut(&mut app)
        .password
        .push_str("sensitive-debug-marker");
    for visible in [false, true] {
        login_mut(&mut app).show_password = visible;
        assert!(!format!("{:?}", login(&app)).contains("sensitive-debug-marker"));
        assert!(!format!("{app:?}").contains("sensitive-debug-marker"));
    }
}

#[test]
fn hiding_a_password_removes_plaintext_from_the_previous_frame() {
    let mut state = login_state();
    state.password.push_str("private-password");
    state.show_password = true;
    let mut terminal = Terminal::new(TestBackend::new(120, 60)).unwrap();
    terminal
        .draw(|frame| ui::render_login_screen(&state, frame))
        .unwrap();
    assert!(buffer_text(terminal.backend().buffer()).contains("private-password"));
    state.show_password = false;
    terminal
        .draw(|frame| ui::render_login_screen(&state, frame))
        .unwrap();
    let hidden = buffer_text(terminal.backend().buffer());
    assert!(!hidden.contains("private-password"));
    assert!(hidden.contains(&"*".repeat(state.password.chars().count())));
}

#[test]
fn resizing_and_redrawing_preserves_the_draft_and_keeps_it_visible() {
    let app = composer("keep my draft", ConnectionStatus::Disconnected);
    let Screen::Main(state) = &app.screen else {
        unreachable!()
    };
    let mut terminal = Terminal::new(TestBackend::new(120, 60)).unwrap();
    terminal
        .draw(|frame| ui::render_main_screen(state, frame))
        .unwrap();
    terminal.backend_mut().resize(80, 24);
    terminal.autoresize().unwrap();
    terminal
        .draw(|frame| ui::render_main_screen(state, frame))
        .unwrap();
    assert_eq!(state.input, "keep my draft");
    assert!(buffer_text(terminal.backend().buffer()).contains("keep my draft"));
}
