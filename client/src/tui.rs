use std::{io, panic};

use crossterm::{event::DisableMouseCapture, terminal::LeaveAlternateScreen};
use ratatui::crossterm::{
    event::EnableMouseCapture,
    execute,
    terminal::{self, EnterAlternateScreen},
};

use anyhow::Result;

pub type CrosstermTerminal = ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stderr>>;

use crate::{
    app::{App, LoginState, MainState, Screen},
    app_event::EventHandler,
    ui,
};

/// representation of a terminal user interface
///
/// handels terminal init and handling the draw events
pub struct Tui {
    terminal: CrosstermTerminal,
    pub events: EventHandler,
}

impl Tui {
    /// constructs a new instance of [`Tui`].
    pub fn new(terminal: CrosstermTerminal, events: EventHandler) -> Self {
        Self { terminal, events }
    }

    /// initializes the terminal interface
    ///
    /// it enables the raw mode and sets terminal properties
    pub fn enter(&mut self) -> Result<()> {
        terminal::enable_raw_mode()?;
        execute!(io::stderr(), EnterAlternateScreen, EnableMouseCapture)?;

        // define a custom panic hook to reset the terminal properties
        // this way, you won't have your terminal messed up if an unexpected error happens
        let panic_hook = panic::take_hook();
        panic::set_hook(Box::new(move |panic| {
            Self::reset().expect("failed to reset the terminal");
            panic_hook(panic);
        }));

        self.terminal.hide_cursor()?;
        self.terminal.clear()?;
        Ok(())
    }

    /// high level abstraction over `terminal.draw()`
    pub fn draw(&mut self, app: &mut App) -> Result<()> {
        match &app.screen {
            Screen::Loading => self.render_loading_screen(),
            Screen::LogIn(login_state) => self.render_login_screen(login_state),
            Screen::Main(main_state) => self.render_main_screen(main_state),
        };

        Ok(())
    }

    /// resets the terminal interface
    fn reset() -> Result<()> {
        terminal::disable_raw_mode()?;
        execute!(io::stderr(), LeaveAlternateScreen, DisableMouseCapture)?;
        Ok(())
    }

    /// exits the terminal interface
    pub fn exit(&mut self) -> Result<()> {
        Self::reset()?;
        self.terminal.show_cursor()?;
        Ok(())
    }

    fn render_loading_screen(&mut self) -> Result<()> {
        self.terminal
            .draw(|frame| ui::render_loading_screen(frame))?;
        Ok(())
    }

    fn render_main_screen(&mut self, state: &MainState) -> Result<()> {
        self.terminal
            .draw(|frame| ui::render_main_screen(state, frame))?;
        Ok(())
    }

    fn render_login_screen(&mut self, state: &LoginState) -> Result<()> {
        self.terminal
            .draw(|frame| ui::render_login_screen(state, frame))?;
        Ok(())
    }

    fn render_register_screen(&mut self, state: LoginState) -> Result<()> {
        todo!()
    }
}
