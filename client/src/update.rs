use anyhow::Error;
use anyhow::anyhow;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use anyhow::Result;

use crate::app::App;
use crate::app::AppCommand;
use crate::app::LoginField;
use crate::app::LoginState;
use crate::app::Screen;
use crate::app::Screen::Main;

pub fn update(app: &mut App, key_event: KeyEvent) -> Option<AppCommand> {
    match key_event.code {
        KeyCode::Esc => app.should_quit = true,

        KeyCode::Char('c') | KeyCode::Char('C') if key_event.modifiers == KeyModifiers::CONTROL => {
            app.should_quit = true
        }

        _ => {
            match &mut app.screen {
                Screen::Loading => {}

                Screen::LogIn(login_state) => {
                    match handle_login_key_event(login_state, key_event) {
                        Some(event) => return Some(event),
                        None => {}
                    }
                }

                Screen::Main(main_state) => todo!(),
            };
        }
    };
    None
}

fn handle_login_key_event(state: &mut LoginState, key_event: KeyEvent) -> Option<AppCommand> {
    match key_event.code {
        KeyCode::Tab => {
            match state.focused_field {
                LoginField::Username => state.focused_field = LoginField::Password,
                LoginField::Password => state.focused_field = LoginField::Username,
            };
        }

        KeyCode::Backspace => {
            match state.focused_field {
                LoginField::Username => state.username.pop(),
                LoginField::Password => state.password.pop(),
            };
        }

        KeyCode::Char(' ') => state.show_password = !state.show_password,

        KeyCode::Char(to_process) => {
            match state.focused_field {
                LoginField::Username => {
                    if state.username.len() < state.username.capacity() {
                        state.username.push(to_process);
                    }
                }

                LoginField::Password => {
                    if state.password.len() < state.password.capacity() {
                        state.password.push(to_process);
                    }
                }
            };
        }

        KeyCode::Enter => match process_input(state) {
            Ok(_) => return Some(AppCommand::LogIn),
            Err(_) => {}
        },

        _ => {}
    };
    None
}

fn process_input(state: &mut LoginState) -> Result<()> {
    if state.username.is_empty() && state.password.is_empty() {
        state.message = Some(String::from("You haven't entered anything!"));
    } else if !state.username.is_empty() && state.password.is_empty() {
        state.message = Some(String::from("You need to enter password as well!"));
    } else if state.username.is_empty() && !state.password.is_empty() {
        state.message = Some(String::from("No username was given!"));
    } else {
        return Ok(());
    }
    Err(anyhow!(""))
}
