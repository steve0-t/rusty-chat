#[derive(Debug)]
pub struct App {
    pub should_quit: bool,
    pub connection: ConnectionStatus,
    pub screen: Screen,
    // pub app_event: AppEvent,
    // pub username: String,
}

#[derive(Debug)]
pub enum AppCommand {
    LogIn,
}

#[derive(Debug)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
    Connecting,
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new() -> Self {
        // todo!("Implement app constructor")
        Self {
            should_quit: false,
            connection: ConnectionStatus::Disconnected,
            screen: Screen::Loading,
        }
    }

    /// Handles the tick event of the terminal.
    pub fn tick(&self) {}

    /// Set should_quit to true to quit the application.
    pub fn quit(&mut self) {
        self.should_quit = true;
    }
}

#[derive(Debug)]
pub enum Screen {
    Loading,
    LogIn(LoginState),
    Main(MainState),
    // Register(RegisterState),
}

#[derive(Debug)]
pub enum InputMode {
    Normal,
    Editing,
}

#[derive(Debug)]
pub struct LoginState {
    pub username: String,
    pub password: String,
    pub focused_field: LoginField,
    pub show_password: bool,
    pub message: Option<String>,
}

#[derive(Debug)]
pub struct MainState {
    pub input: String,
}

#[derive(Debug)]
pub enum LoginField {
    Username,
    Password,
}
