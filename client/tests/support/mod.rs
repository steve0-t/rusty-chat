//! Initially generated with AI assistance on 2026-10-21.
#![allow(dead_code)]

use client::app::{App, ConnectionStatus, LoginField, LoginState, Screen};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

pub fn login_state() -> LoginState {
    LoginState {
        username: String::with_capacity(128),
        password: String::with_capacity(128),
        focused_field: LoginField::Username,
        show_password: false,
        message: None,
    }
}

pub fn login_app() -> App {
    App {
        should_quit: false,
        connection: ConnectionStatus::Disconnected,
        screen: Screen::LogIn(login_state()),
    }
}

pub fn login(app: &App) -> &LoginState {
    match &app.screen {
        Screen::LogIn(state) => state,
        screen => panic!("expected login screen, got {screen:?}"),
    }
}

pub fn login_mut(app: &mut App) -> &mut LoginState {
    match &mut app.screen {
        Screen::LogIn(state) => state,
        screen => panic!("expected login screen, got {screen:?}"),
    }
}

pub fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

pub fn render(width: u16, height: u16, draw: impl FnOnce(&mut ratatui::Frame)) -> Buffer {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("construct headless terminal");
    terminal.draw(draw).expect("render to headless terminal");
    terminal.backend().buffer().clone()
}

pub fn buffer_text(buffer: &Buffer) -> String {
    (buffer.area.y..buffer.area.bottom())
        .map(|y| {
            let mut row = String::new();
            let mut x = buffer.area.x;
            while x < buffer.area.right() {
                let symbol = buffer[(x, y)].symbol();
                row.push_str(symbol);
                // Ratatui reserves extra cells after wide glyphs. Skip those
                // continuation cells so visible Unicode text stays contiguous.
                let width = ratatui::text::Line::raw(symbol).width().max(1);
                x = x.saturating_add(width as u16);
            }
            row
        })
        .collect::<Vec<_>>()
        .join("\n")
}
