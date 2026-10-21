use std::{ops::DerefMut, thread::sleep, time::Duration};

use crossterm::style::Stylize;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect, Spacing},
    style::{
        Color::{self, Gray, Yellow},
        Style, Stylize,
    },
    text::{Line, Text},
    widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph, canvas::Label},
};

use crate::app::{App, LoginField, LoginState, MainState, Screen};

pub fn render_loading_screen(frame: &mut Frame) {}

pub fn render_login_screen(state: &LoginState, frame: &mut Frame) {
    let centered_area = frame
        .area()
        .centered(Constraint::Max(75), Constraint::Percentage(20));

    let centered_chunks =
        Layout::vertical([Constraint::Percentage(80), Constraint::Percentage(20)])
            .split(centered_area);
    frame.render_widget(Clear, centered_area);

    let instructions = Line::from(vec![
        " Change focus ".into(),
        "<Tab>".yellow().bold(),
        " Show password ".into(),
        "<Space>".yellow().bold(),
        " Submit ".into(),
        "<Enter>".green().bold(),
        " Quit ".into(),
        "<Esc> ".red().bold(),
    ])
    .centered();

    frame.render_widget(instructions, centered_chunks[1]);

    let title = Line::from(" Log in ".bold());
    let popup_block = Block::bordered().title_top(title.centered());

    let inner_area = popup_block.inner(centered_chunks[0]);
    let chunks = Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner_area);

    let username_to_render = if state.username.len() > chunks[0].width as usize {
        state.username.get(state.username.len()..).unwrap()
    } else {
        state.username.as_str()
    };

    let actual_username = Paragraph::new(username_to_render).block(match state.focused_field {
        LoginField::Username => Block::bordered().title("Username").border_style(Yellow),
        LoginField::Password => Block::bordered().title("Username").border_style(Gray),
    });

    frame.render_widget(actual_username, chunks[0]);

    let pswd_to_display = match state.show_password {
        true => state.password.as_str(),
        false => {
            let res = &"*".repeat(state.password.len());
            // res.as_str()
            &res.to_string()
        }
    };

    let actual_password = Paragraph::new(pswd_to_display).block(match state.focused_field {
        LoginField::Username => Block::bordered().title("Password").border_style(Gray),
        LoginField::Password => Block::bordered().title("Password").border_style(Yellow),
    });

    frame.render_widget(actual_password, chunks[1]);

    // render the input field container
    frame.render_widget(popup_block, centered_chunks[0]);

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
