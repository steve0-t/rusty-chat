use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::App;
use crate::app::InputMode;
use crate::app::LoginField;
use crate::app::LoginState;
use crate::app::Screen;

pub fn update(app: &mut App, key_event: KeyEvent) {
    match &mut app.screen {
        Screen::Loading => {}

        Screen::LogIn(login_state) => {
            handle_login_key_event(login_state, key_event);
        }

        Screen::Main(main_state) => todo!(),
    };
}

fn handle_login_key_event(state: &mut LoginState, key_event: KeyEvent) {
    match key_event.code {
        KeyCode::Tab => match state.focused_field {
            LoginField::Username => state.focused_field = LoginField::Password,
            LoginField::Password => state.focused_field = LoginField::Username,
        },

        _ => {}
    }
}
