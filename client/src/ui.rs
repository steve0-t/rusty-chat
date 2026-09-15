use std::{ops::DerefMut, thread::sleep, time::Duration};

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph},
};

use crate::app::App;

pub fn render(app: &mut App, frame: &mut Frame) {
    let layout = Layout::horizontal([
        Constraint::Percentage(25),
        Constraint::Percentage(60),
        Constraint::Percentage(15),
    ])
    .split(frame.area());

    if app.get_input {
        render_box_with_fields(app, frame);
        return;
    }

    render_left_chunks(app, frame, layout[0]);

    let curr_chat = Block::bordered().title("<Name of chat>");
    frame.render_widget(curr_chat, layout[1]);

    let emojis = Block::bordered().title("<Emojis>");
    frame.render_widget(emojis, layout[2]);

    if !app.connection_available {
        render_connection_unavailable(frame);
    }

    // let text = Text::from(Line::from(app.in_buf.as_str()));
    // frame.render_widget(text, frame.area());
}

fn render_left_chunks(app: &mut App, frame: &mut Frame, area: Rect) {
    let chunks = Layout::vertical([Constraint::Percentage(5), Constraint::Percentage(95)])
        // .spacing(-1)
        .split(area);

    if app.display_user {
        render_user(app, frame, chunks[0]);
    }

    let chats = Block::bordered().title("Chats");
    frame.render_widget(chats, chunks[1]);
}

fn render_user(app: &mut App, frame: &mut Frame, rect: Rect) {
    let block = Block::bordered();
    let username_text = format!("Logged in as {}", app.username);
    let username = Paragraph::new(username_text).block(block);
    frame.render_widget(username, rect);
}

// pub fn render_input_screen(fields: [&str; N]) {}

fn render_box_with_fields(app: &mut App, frame: &mut Frame) {
    frame.render_widget(Clear, frame.area());

    let mut title = "Input".to_string();
    if !app.log_in && app.register {
        title = "Register".to_string();
    } else if app.log_in && !app.register {
        title = "Log in".to_string()
    }

    let input_block = Block::bordered().title(title);
    frame.render_widget(input_block, frame.area());
}

// fn draw_first_tab(frame: &mut Frame, app: &mut App, area: Rect) {
//     let chats = Block::bordered().title("Chats");
//     frame.render_widget(chats, area);

//     let curr_chat = Block::bordered().title("<Name of chat>");
// }

fn render_connection_unavailable(frame: &mut Frame) {}
