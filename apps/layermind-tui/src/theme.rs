#![allow(dead_code)]
use ratatui::style::{Color, Modifier, Style};

const PANEL: Color = Color::Rgb(16, 18, 28);
const FG: Color = Color::Rgb(200, 205, 215);
const MUTED: Color = Color::Rgb(80, 85, 100);
const ACCENT: Color = Color::Rgb(0, 200, 215);
const ACCENT2: Color = Color::Rgb(0, 160, 180);
const OK: Color = Color::Rgb(50, 210, 100);
const WARN: Color = Color::Rgb(230, 180, 30);
const ALERT: Color = Color::Rgb(230, 60, 70);
const HEADER_BG: Color = Color::Rgb(0, 160, 180);
const BORDER: Color = Color::Rgb(40, 48, 65);

pub fn accent() -> Color { ACCENT }
pub fn accent2() -> Color { ACCENT2 }

pub fn panel() -> Color { PANEL }
pub fn fg() -> Color { FG }
pub fn muted() -> Color { MUTED }

pub fn warn() -> Color { WARN }
pub fn ok() -> Color { OK }
pub fn alert() -> Color { ALERT }
pub fn header_bg() -> Color { HEADER_BG }

pub fn block() -> ratatui::widgets::Block<'static> {
    ratatui::widgets::Block::bordered()
        .border_set(ratatui::symbols::border::ROUNDED)
        .border_style(Style::new().fg(BORDER))
        .style(Style::new().bg(PANEL))
}

pub fn header_style() -> Style {
    Style::new()
        .bg(HEADER_BG)
        .fg(Color::Rgb(10, 12, 20))
        .add_modifier(Modifier::BOLD)
}

pub fn status(level: &str) -> Style {
    match level {
        "printing" | "Printing" => Style::new().fg(OK).add_modifier(Modifier::BOLD),
        "paused" | "Paused" => Style::new().fg(WARN).add_modifier(Modifier::BOLD),
        "error" | "Error" | "failed" | "Failed" => Style::new().fg(ALERT).add_modifier(Modifier::BOLD),
        _ => Style::new().fg(MUTED),
    }
}
