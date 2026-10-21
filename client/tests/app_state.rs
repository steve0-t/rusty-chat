mod support;

use client::{
    app::{App, ConnectionStatus, MainState, Screen},
    update::update,
};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use support::{key, login_app, login_mut};

#[test]
fn new_app_starts_running() {
    assert!(!App::new().should_quit);
}

#[test]
fn new_app_starts_disconnected() {
    assert!(matches!(
        App::new().connection,
        ConnectionStatus::Disconnected
    ));
}

#[test]
fn new_app_starts_on_loading_screen() {
    assert!(matches!(App::new().screen, Screen::Loading));
}

#[test]
fn separate_apps_do_not_share_quit_state() {
    let mut first = App::new();
    let second = App::new();
    first.quit();
    assert!(first.should_quit);
    assert!(!second.should_quit);
}

#[test]
fn quit_is_idempotent() {
    let mut app = App::new();
    app.quit();
    app.quit();
    assert!(app.should_quit);
}

#[test]
fn quit_preserves_login_data_and_connection() {
    let mut app = login_app();
    login_mut(&mut app).username.push_str("alice");
    app.connection = ConnectionStatus::Connected;
    let original_screen = format!("{:?}", app.screen);
    app.quit();
    assert_eq!(format!("{:?}", app.screen), original_screen);
    assert!(matches!(app.connection, ConnectionStatus::Connected));
}

#[test]
fn tick_preserves_application_state() {
    let mut app = login_app();
    login_mut(&mut app).password.push_str("secret");
    app.connection = ConnectionStatus::Connecting;
    let before = format!("{app:?}");
    for _ in 0..100 {
        app.tick();
    }
    assert_eq!(format!("{app:?}"), before);
}

#[test]
fn ticking_cannot_resume_an_app_that_has_quit() {
    let mut app = App::new();
    app.quit();
    app.tick();
    assert!(app.should_quit);
}

#[test]
fn loading_screen_ignores_regular_input() {
    let mut app = App::new();
    for code in [
        KeyCode::Char('a'),
        KeyCode::Tab,
        KeyCode::Enter,
        KeyCode::Backspace,
    ] {
        assert!(update(&mut app, key(code)).is_none());
        assert!(matches!(app.screen, Screen::Loading));
        assert!(!app.should_quit);
    }
}

#[test]
fn escape_quits_every_screen_without_emitting_login() {
    for screen in [
        Screen::Loading,
        login_app().screen,
        Screen::Main(MainState {
            input: "draft".into(),
        }),
    ] {
        let mut app = App::new();
        app.screen = screen;
        assert!(update(&mut app, key(KeyCode::Esc)).is_none());
        assert!(app.should_quit);
    }
}

#[test]
fn control_c_quits_every_screen_without_emitting_login() {
    for screen in [
        Screen::Loading,
        login_app().screen,
        Screen::Main(MainState {
            input: String::new(),
        }),
    ] {
        for letter in ['c', 'C'] {
            let mut app = App::new();
            // Each screen is consumed once; the same screen variant is enough for
            // the second case because quit handling happens before screen dispatch.
            app.screen = match &screen {
                Screen::Loading => Screen::Loading,
                Screen::LogIn(_) => login_app().screen,
                Screen::Main(_) => Screen::Main(MainState {
                    input: String::new(),
                }),
            };
            assert!(
                update(
                    &mut app,
                    KeyEvent::new(KeyCode::Char(letter), KeyModifiers::CONTROL)
                )
                .is_none()
            );
            assert!(app.should_quit);
        }
    }
}

#[test]
fn unmodified_c_is_text_in_login() {
    let mut app = login_app();
    update(&mut app, key(KeyCode::Char('c')));
    assert_eq!(support::login(&app).username, "c");
    assert!(!app.should_quit);
}

#[test]
fn shifted_uppercase_c_is_text_in_login() {
    let mut app = login_app();
    update(
        &mut app,
        KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT),
    );
    assert_eq!(support::login(&app).username, "C");
    assert!(!app.should_quit);
}
