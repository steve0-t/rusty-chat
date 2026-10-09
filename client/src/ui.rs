use std::{ops::DerefMut, thread::sleep, time::Duration};

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect, Spacing},
    style::{Color, Style, Stylize},
    text::{Line, Text},
    widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph},
};

use crate::app::{App, LoginField, LoginState, MainState, Screen};

pub fn render_loading_screen(frame: &mut Frame) {}

pub fn render_login_screen(state: &LoginState, frame: &mut Frame) {
    let centered_area = frame
        .area()
        .centered(Constraint::Percentage(20), Constraint::Percentage(30));
    frame.render_widget(Clear, centered_area);

    let popup_block = Block::bordered().title("Log in");

    let inner_area = popup_block.inner(centered_area);
    let chunks = Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner_area);

    // render username input field with title
    let username_field =
        Layout::vertical([Constraint::Percentage(10), Constraint::Percentage(90)]).split(chunks[0]);

    // render password input field with title
    let password_field =
        Layout::vertical([Constraint::Percentage(10), Constraint::Percentage(90)]).split(chunks[1]);

    let paragraph = Paragraph::new("Username");
    frame.render_widget(paragraph, username_field[0]);

    let focused_block = Block::bordered().yellow();

    let not_focused_block = Block::bordered().gray();

    match state.focused_field {
        LoginField::Username => {
            frame.render_widget(focused_block, username_field[1]);
            frame.render_widget(not_focused_block, password_field[1]);
        }

        LoginField::Password => {
            frame.render_widget(not_focused_block, username_field[1]);
            frame.render_widget(focused_block, password_field[1]);
        }
    }

    let paragraph = Paragraph::new("Password");
    frame.render_widget(paragraph, password_field[0]);

    // render the input field container
    frame.render_widget(popup_block, centered_area);

    // let paragraph = Paragraph::new("Password").block(popup_block);
    // frame.render_widget(paragraph, centered_area);

    // another solution is to use the inner area of the block
    // let inner_area = popup_block.inner(centered_area);
    // frame.render_widget(your_widget, inner_area);
}

pub fn render_main_screen(state: &MainState, frame: &mut Frame) {
    let layout = Layout::horizontal([
        Constraint::Percentage(25),
        Constraint::Percentage(60),
        Constraint::Percentage(15),
    ])
    .split(frame.area());

    render_left_chunks(state, frame, layout[0]);

    let curr_chat = Block::bordered().title("<Name of chat>");
    frame.render_widget(curr_chat, layout[1]);

    let emojis = Block::bordered().title("<Emojis>");
    frame.render_widget(emojis, layout[2]);

    // if !state.connection_available {
    //     render_connection_unavailable(frame);
    // }

    // let text = Text::from(Line::from(app.in_buf.as_str()));
    // frame.render_widget(text, frame.area());
}

fn render_left_chunks(state: &MainState, frame: &mut Frame, area: Rect) {
    let chunks = Layout::vertical([Constraint::Percentage(5), Constraint::Percentage(95)])
        // .spacing(-1)
        .split(area);

    render_user(state, frame, chunks[0]);

    let chats = Block::bordered().title("Chats");
    frame.render_widget(chats, chunks[1]);
}

fn render_user(state: &MainState, frame: &mut Frame, rect: Rect) {
    let block = Block::bordered();
    let username_text = format!("Logged in as {}", "lol");
    let username = Paragraph::new(username_text).block(block);
    frame.render_widget(username, rect);
}

// pub fn render_input_screen(fields: [&str; N]) {}

// fn render_box_with_fields(app: &mut App, frame: &mut Frame) {
//     frame.render_widget(Clear, frame.area());

//     let mut title = "Input".to_string();
//     if !app.log_in && app.register {
//         title = "Register".to_string();
//     } else if app.log_in && !app.register {
//         title = "Log in".to_string()
//     }

//     let input_block = Block::bordered().title(title);
//     frame.render_widget(input_block, frame.area());
// }

// fn draw_first_tab(frame: &mut Frame, app: &mut App, area: Rect) {
//     let chats = Block::bordered().title("Chats");
//     frame.render_widget(chats, area);

//     let curr_chat = Block::bordered().title("<Name of chat>");
// }

fn render_connection_unavailable(frame: &mut Frame) {}
