use std::ops::DerefMut;

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Padding, Paragraph},
};

use crate::app::App;

pub fn render(app: &mut App, frame: &mut Frame) {
    let layout = Layout::horizontal([
        Constraint::Percentage(25),
        Constraint::Percentage(60),
        Constraint::Percentage(15),
    ])
    .split(frame.area());

    let chats = Block::bordered().title("Chats");
    frame.render_widget(chats, layout[0]);

    let curr_chat = Block::bordered().title("<Name of chat>");
    frame.render_widget(curr_chat, layout[1]);

    let emojis = Block::bordered().title("<Emojis>");
    frame.render_widget(emojis, layout[2]);
}

// fn draw_first_tab(frame: &mut Frame, app: &mut App, area: Rect) {
//     let chats = Block::bordered().title("Chats");
//     frame.render_widget(chats, area);

//     let curr_chat = Block::bordered().title("<Name of chat>");
// }
