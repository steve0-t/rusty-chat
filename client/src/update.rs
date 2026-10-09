use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::App;
use crate::app::AppState;

pub fn update(app: &mut App, key_event: KeyEvent) {
    match app.app_state {
        AppState::Main => todo!(),
        AppState::LogIn => handle_login_key_event(app, key_event),
        AppState::Register => todo!(),
    };
}

fn handle_login_key_event(app: &mut App, key_event: KeyEvent) {
    match key_event.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('c') | KeyCode::Char('C')
            if key_event.modifiers == KeyModifiers::CONTROL =>
        {
            app.quit()
        }

        KeyCode::Char('n') => {
            app.login_change_focus = !app.login_change_focus;
        }

        KeyCode::Char('i') => {
            app.login_getting_input = true;
        }

        KeyCode::Char(to_process) => {
            app.in_buf.push(to_process);
        }

        _ => {}
    };
}
