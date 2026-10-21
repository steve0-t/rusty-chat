mod support;

use client::{
    app::{LoginField, MainState},
    ui,
};
use ratatui::{buffer::Buffer, style::Color};
use support::{buffer_text, login_state, render};

fn login_buffer(state: &client::app::LoginState) -> Buffer {
    render(120, 60, |frame| ui::render_login_screen(state, frame))
}

fn border_color_for_label(buffer: &Buffer, label: &str) -> Color {
    for y in 0..buffer.area.height {
        let row = (0..buffer.area.width)
            .map(|x| buffer[(x, y)].symbol())
            .collect::<String>();
        if row.contains(label) {
            let x = row
                .chars()
                .position(|ch| ch == '┌')
                .expect("field top border") as u16;
            return buffer[(x, y)].fg;
        }
    }
    panic!("missing field label {label}");
}

#[test]
fn login_screen_shows_title_and_both_fields() {
    let text = buffer_text(&login_buffer(&login_state()));
    for label in ["Log in", "Username", "Password"] {
        assert!(text.contains(label), "missing {label}:\n{text}");
    }
}

#[test]
fn login_screen_explains_its_keyboard_shortcuts() {
    let text = buffer_text(&login_buffer(&login_state()));
    for shortcut in ["<Tab>", "<Space>", "<Enter>", "<Esc>"] {
        assert!(text.contains(shortcut), "missing {shortcut}:\n{text}");
    }
}

#[test]
fn username_is_visible() {
    let mut state = login_state();
    state.username.push_str("alice_42");
    assert!(buffer_text(&login_buffer(&state)).contains("alice_42"));
}

#[test]
fn unicode_username_is_visible() {
    let mut state = login_state();
    state.username.push_str("Žofia界");
    assert!(buffer_text(&login_buffer(&state)).contains("Žofia界"));
}

#[test]
fn hidden_ascii_password_is_masked_without_exposing_plaintext() {
    let mut state = login_state();
    state.password.push_str("unique_secret");
    let text = buffer_text(&login_buffer(&state));
    assert!(!text.contains("unique_secret"));
    assert!(text.contains(&"*".repeat("unique_secret".len())));
}

#[test]
fn shown_password_is_visible_without_mask() {
    let mut state = login_state();
    state.password.push_str("unique_secret");
    state.show_password = true;
    let text = buffer_text(&login_buffer(&state));
    assert!(text.contains("unique_secret"));
    assert!(!text.contains('*'));
}

#[test]
fn shown_unicode_password_is_visible() {
    let mut state = login_state();
    state.password.push_str("密碼🦀");
    state.show_password = true;
    assert!(buffer_text(&login_buffer(&state)).contains("密碼🦀"));
}

#[test]
fn empty_password_has_no_mask_characters() {
    let text = buffer_text(&login_buffer(&login_state()));
    assert!(!text.contains('*'));
}

#[test]
fn username_focus_highlights_only_username_border() {
    let buffer = login_buffer(&login_state());
    assert_eq!(border_color_for_label(&buffer, "Username"), Color::Yellow);
    assert_eq!(border_color_for_label(&buffer, "Password"), Color::Gray);
}

#[test]
fn password_focus_moves_border_highlight() {
    let mut state = login_state();
    state.focused_field = LoginField::Password;
    let buffer = login_buffer(&state);
    assert_eq!(border_color_for_label(&buffer, "Username"), Color::Gray);
    assert_eq!(border_color_for_label(&buffer, "Password"), Color::Yellow);
}

#[test]
fn login_render_does_not_mutate_credentials_or_visibility() {
    let mut state = login_state();
    state.username.push_str("alice");
    state.password.push_str("secret");
    state.message = Some("validation error".into());
    let before = format!("{state:?}");
    login_buffer(&state);
    assert_eq!(format!("{state:?}"), before);
}

#[test]
fn main_screen_draws_chat_list_current_chat_and_emoji_panes() {
    let state = MainState {
        input: String::new(),
    };
    let text = buffer_text(&render(120, 60, |frame| {
        ui::render_main_screen(&state, frame)
    }));
    for label in ["Chats", "<Name of chat>", "<Emojis>"] {
        assert!(text.contains(label), "missing {label}:\n{text}");
    }
}

#[test]
fn main_render_does_not_mutate_draft() {
    let state = MainState {
        input: "unsent draft 🦀".into(),
    };
    render(120, 60, |frame| ui::render_main_screen(&state, frame));
    assert_eq!(state.input, "unsent draft 🦀");
}

#[test]
fn all_screens_render_at_zero_tiny_and_standard_terminal_sizes() {
    let login = login_state();
    let main = MainState {
        input: String::new(),
    };
    for (width, height) in [
        (0, 0),
        (0, 1),
        (1, 0),
        (1, 1),
        (2, 2),
        (10, 5),
        (40, 10),
        (80, 24),
        (120, 60),
    ] {
        render(width, height, ui::render_loading_screen);
        render(width, height, |frame| {
            ui::render_login_screen(&login, frame)
        });
        render(width, height, |frame| ui::render_main_screen(&main, frame));
    }
}

#[test]
fn long_unicode_input_does_not_panic_at_small_sizes() {
    let mut state = login_state();
    state.username = "ž界🦀".repeat(128);
    state.password = "é🦀".repeat(128);
    for (width, height) in [(1, 1), (10, 5), (80, 24), (120, 60)] {
        render(width, height, |frame| {
            ui::render_login_screen(&state, frame)
        });
        state.show_password = true;
        render(width, height, |frame| {
            ui::render_login_screen(&state, frame)
        });
        state.show_password = false;
    }
}

#[test]
fn repeated_login_render_is_deterministic() {
    let mut state = login_state();
    state.username.push_str("alice");
    state.password.push_str("secret");
    assert_eq!(login_buffer(&state), login_buffer(&state));
}

#[test]
fn toggling_visibility_changes_rendered_password() {
    let mut state = login_state();
    state.password.push_str("secret");
    let hidden = login_buffer(&state);
    state.show_password = true;
    let shown = login_buffer(&state);
    assert_ne!(hidden, shown);
    assert!(!buffer_text(&hidden).contains("secret"));
    assert!(buffer_text(&shown).contains("secret"));
}
