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
    }

    let chats = Block::bordered().title("Chats");
    frame.render_widget(chats, layout[0]);

    let curr_chat = Block::bordered().title("<Name of chat>");
    frame.render_widget(curr_chat, layout[1]);

    let emojis = Block::bordered().title("<Emojis>");
    frame.render_widget(emojis, layout[2]);

    if app.log_in {
        frame.render_widget(Clear, frame.area());
        draw_log_in(app, frame)
    }

    // let text = Text::from(Line::from(app.in_buf.as_str()));
    // frame.render_widget(text, frame.area());
}

fn draw_log_in(app: &mut App, frame: &mut Frame) {
    let emojis = Block::bordered().title("<Emojis>");
    frame.render_widget(emojis, frame.area());
}

fn render_box_with_fields(app: &mut App, frame: &mut Frame) {}

// fn draw_first_tab(frame: &mut Frame, app: &mut App, area: Rect) {
//     let chats = Block::bordered().title("Chats");
//     frame.render_widget(chats, area);

//     let curr_chat = Block::bordered().title("<Name of chat>");
// }
