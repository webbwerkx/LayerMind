use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Paragraph},
};
use crate::app::AppState;
use crate::theme;

pub fn render(f: &mut Frame, app: &mut AppState) {
    let area = f.area();

    if area.width < 80 || area.height < 20 {
        let msg = format!(
            "Terminal too small ({}x{}). Minimum 80x24.",
            area.width, area.height
        );
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(msg, Style::new().fg(theme::warn()))))
                .style(Style::new().bg(theme::panel())),
            area,
        );
        return;
    }

    let vert = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(area);

    render_header(f, vert[0], app);
    render_body(f, vert[1], app);
    render_footer(f, vert[2], app);

    if app.show_machine {
        if let Some(ref profile) = app.machine_profile {
            render_machine_popup(f, area, profile);
        }
    }
}

fn render_header(f: &mut Frame, area: Rect, app: &AppState) {
    let elapsed = if app.printer.print_elapsed > 0.0 {
        format_duration(app.printer.print_elapsed as u64)
    } else {
        "--:--:--".into()
    };
    let layer_info = app.printer.current_layer.map(|l| {
        if let Some(total) = app.printer.total_layers {
            format!("{:>3}/{}", l, total)
        } else {
            format!("{:>3}", l)
        }
    });
    let status = app.status_text().to_uppercase();
    let status_style = theme::status(&status);
    let host = app.printer.hostname.as_deref().unwrap_or("--");

    let mut spans = vec![
        Span::styled(" ◉ ", Style::new().fg(theme::accent())),
        Span::styled("LAYERMIND ", theme::header_style()),
        Span::styled(" ◆ ", Style::new().fg(theme::muted())),
        Span::styled(host, theme::header_style()),
        Span::styled(" ◆ ", Style::new().fg(theme::muted())),
        Span::styled(status, status_style),
    ];
    let right = format!(" {}  {} ", elapsed, layer_info.unwrap_or_default());
    let pad = area.width.saturating_sub(45).max(1) as usize;
    if !right.trim().is_empty() {
        spans.push(Span::raw(" ".repeat(pad)));
        spans.push(Span::styled(right, theme::header_style()));
    }
    f.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::new().bg(theme::header_bg())),
        area,
    );
}

fn render_body(f: &mut Frame, area: Rect, app: &mut AppState) {
    let cols = Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(area);
    render_left(f, cols[0], app);
    render_right(f, cols[1], app);
}

fn render_left(f: &mut Frame, area: Rect, app: &mut AppState) {
    let vert = Layout::vertical([Constraint::Length(8), Constraint::Min(3)]).split(area);
    render_state_panel(f, vert[0], app);
    render_events_panel(f, vert[1], app);
}

fn render_right(f: &mut Frame, area: Rect, app: &mut AppState) {
    let vert = Layout::vertical([
        Constraint::Length(7),
        Constraint::Length(5),
        Constraint::Min(3),
    ])
    .split(area);
    render_temps_panel(f, vert[0], app);
    render_progress_panel(f, vert[1], app);
    render_recs_panel(f, vert[2], app);
}

fn render_state_panel(f: &mut Frame, area: Rect, app: &AppState) {
    let block = theme::block().title(" STATE ");
    let inner = block.inner(area);
    f.render_widget(block, area);
    let state = app.printer.state.to_uppercase();
    let state_style = theme::status(&state);
    let rows = vec![
        Line::from(vec![
            Span::raw("  Host:     "),
            Span::styled(app.printer.hostname.as_deref().unwrap_or("--"), Style::new().fg(theme::fg())),
        ]),
        Line::from(vec![Span::raw("  Status:   "), Span::styled(state, state_style)]),
        Line::from(vec![
            Span::raw("  Print:    "),
            Span::styled(app.printer.print_filename.as_deref().unwrap_or("--"), Style::new().fg(theme::accent())),
        ]),
        Line::from(vec![
            Span::raw("  Progress: "),
            Span::styled(format!("{:.1}%", app.printer.print_progress * 100.0), Style::new().fg(theme::accent2())),
        ]),
        Line::from(vec![
            Span::raw("  Position: "),
            Span::styled(
                format!("X{:.1} Y{:.1} Z{:.1}", app.printer.position[0], app.printer.position[1], app.printer.position[2]),
                Style::new().fg(theme::fg()),
            ),
        ]),
        Line::from(vec![
            Span::raw("  Speed:    "),
            Span::styled(format!("{:.0} mm/s", app.printer.speed), Style::new().fg(theme::fg())),
        ]),
    ];
    for (i, row) in rows.iter().enumerate() {
        if i as u16 >= inner.height { break; }
        let row_area = Rect { x: inner.x + 1, y: inner.y + i as u16, width: inner.width.saturating_sub(2), height: 1 };
        f.render_widget(Paragraph::new(row.clone()), row_area);
    }
}

fn render_events_panel(f: &mut Frame, area: Rect, app: &mut AppState) {
    let block = theme::block().title(" EVENTS ");
    let inner = block.inner(area);
    f.render_widget(block, area);
    if app.events.is_empty() {
        f.render_widget(Paragraph::new("  No events yet").style(Style::new().fg(theme::muted())), inner);
        return;
    }
    let scroll = app.event_scroll;
    let visible = inner.height as usize;
    let start = scroll.saturating_sub(visible.saturating_sub(1));
    let end = (start + visible).min(app.events.len());
    for (i, entry) in app.events[start..end].iter().enumerate() {
        let (icon, color) = match entry.level {
            crate::app::EventLevel::Info => (" ◆", theme::fg()),
            crate::app::EventLevel::Warning => (" ⚠", theme::warn()),
            crate::app::EventLevel::Error => (" ✗", theme::alert()),
        };
        let row_area = Rect { x: inner.x + 1, y: inner.y + i as u16, width: inner.width.saturating_sub(2), height: 1 };
        f.render_widget(Paragraph::new(Line::from(Span::styled(icon, Style::new().fg(color)))), row_area);
        let max_w = inner.width.saturating_sub(6) as usize;
        let text: String = if entry.message.len() > max_w {
            format!("{}…", &entry.message[..max_w.saturating_sub(1)])
        } else {
            entry.message.clone()
        };
        let text_area = Rect { x: inner.x + 3, y: inner.y + i as u16, width: inner.width.saturating_sub(4), height: 1 };
        f.render_widget(Paragraph::new(Line::from(Span::raw(format!(" {text}")))).style(Style::new().fg(color)), text_area);
    }
}

fn render_temps_panel(f: &mut Frame, area: Rect, app: &AppState) {
    let block = theme::block().title(" TEMPERATURES ");
    let inner = block.inner(area);
    f.render_widget(block, area);
    let ext_pct = if app.printer.extruder_target > 0.0 { (app.printer.extruder_temp / app.printer.extruder_target).min(1.0) } else { 0.0 };
    let bed_pct = if app.printer.bed_target > 0.0 { (app.printer.bed_temp / app.printer.bed_target).min(1.0) } else { 0.0 };
    let rows = Layout::vertical([Constraint::Length(2), Constraint::Length(1), Constraint::Length(2), Constraint::Length(1)]).split(inner);
    render_gauge_label(f, rows[0], "Extruder", app.printer.extruder_temp, app.printer.extruder_target, ext_pct, theme::accent());
    render_gauge_bar(f, rows[1], ext_pct, theme::accent());
    render_gauge_label(f, rows[2], "Bed", app.printer.bed_temp, app.printer.bed_target, bed_pct, theme::accent2());
    render_gauge_bar(f, rows[3], bed_pct, theme::accent2());
}

use ratatui::style::Color;

fn render_gauge_label(f: &mut Frame, area: Rect, label: &str, current: f64, target: f64, _pct: f64, color: Color) {
    let text = if target > 0.0 {
        format!("  {label:<10} {current:>5.0}°C / {target:>5.0}°C")
    } else {
        format!("  {label:<10} {current:>5.0}°C")
    };
    f.render_widget(Paragraph::new(Line::from(Span::styled(text, Style::new().fg(color)))), area);
}

fn render_gauge_bar(f: &mut Frame, area: Rect, pct: f64, color: Color) {
    let w = area.width.saturating_sub(4) as usize;
    if w == 0 { return; }
    let filled = (pct * w as f64).round() as usize;
    let filled = filled.min(w);
    let empty = w.saturating_sub(filled);
    let bar = format!("  {}{}", "█".repeat(filled), "░".repeat(empty));
    f.render_widget(Paragraph::new(Line::from(Span::styled(bar, Style::new().fg(color)))), area);
}

fn render_progress_panel(f: &mut Frame, area: Rect, app: &AppState) {
    let block = theme::block().title(" PROGRESS ");
    let inner = block.inner(area);
    f.render_widget(block, area);
    let cols = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).split(inner);
    let w = inner.width.saturating_sub(4) as usize;
    let pct = app.printer.print_progress;
    let filled = (pct * w as f64).round() as usize;
    let filled = filled.min(w);
    let empty = w.saturating_sub(filled);
    let bar = format!("  {}{}", "▓".repeat(filled), "░".repeat(empty));
    f.render_widget(Paragraph::new(Line::from(Span::styled(bar, Style::new().fg(theme::accent())))), cols[0]);
    let layer_str = match (app.printer.current_layer, app.printer.total_layers) {
        (Some(c), Some(t)) => format!("Layer {}/{}", c, t),
        (Some(c), None) => format!("Layer {}", c),
        _ => String::new(),
    };
    let eta_str = app.printer.print_remaining.map(|s| format!("ETA {}", format_duration(s as u64))).unwrap_or_default();
    let info = format!("  {:>5.1}%   {}   {}", pct * 100.0, layer_str, eta_str);
    f.render_widget(Paragraph::new(Line::from(Span::styled(info, Style::new().fg(theme::fg())))), cols[1]);
}

fn render_recs_panel(f: &mut Frame, area: Rect, app: &AppState) {
    let block = theme::block().title(" DIAGNOSTICS ");
    let inner = block.inner(area);
    f.render_widget(block, area);
    if app.running_diagnostic {
        f.render_widget(Paragraph::new("  Running AI diagnostic...").style(Style::new().fg(theme::accent())), inner);
        return;
    }
    if let Some(ref err) = app.diagnostic_error {
        f.render_widget(Paragraph::new(format!("  Error: {err}")).style(Style::new().fg(theme::alert())), inner);
        return;
    }
    if let Some(ref result) = app.diagnostic_result {
        let mut lines = vec![
            Line::from(Span::styled(format!("  {}", result.recommendation.summary), Style::new().fg(theme::fg()))),
            Line::from(Span::styled(format!("  Confidence: {:.2}", result.recommendation.confidence), Style::new().fg(theme::accent2()))),
            Line::from(Span::raw("")),
        ];
        for (i, action) in result.recommendation.actions.iter().enumerate() {
            if i >= inner.height as usize - 3 { break; }
            let cmd = action.suggested_command.as_deref().unwrap_or("");
            let line = if cmd.is_empty() {
                format!("  {}. {}", i + 1, action.description)
            } else {
                format!("  {}. {} [{cmd}]", i + 1, action.description)
            };
            lines.push(Line::from(Span::styled(line, Style::new().fg(theme::fg()))));
        }
        f.render_widget(Paragraph::new(lines), inner);
        return;
    }
    let msg = Paragraph::new(Line::from(vec![
        Span::raw("  Press "),
        Span::styled("d", Style::new().fg(theme::accent())),
        Span::raw(" to run AI diagnostic"),
    ]))
    .style(Style::new().fg(theme::muted()));
    f.render_widget(msg, inner);
}

fn render_footer(f: &mut Frame, area: Rect, app: &AppState) {
    let mut hints = vec![" q:quit ", " d:diagnose ", " m:machine "];
    if app.show_machine { hints.push(" M:close "); }
    if app.connected { hints.push(" TAB:focus "); }
    let text = hints.join("│");
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(text, Style::new().fg(theme::muted()))))
            .style(Style::new().bg(theme::panel())),
        area,
    );
}

fn render_machine_popup(f: &mut Frame, area: Rect, profile: &layermind_shared::machine::MachineProfile) {
    let popup_area = Rect {
        x: area.width.saturating_sub(70) / 2,
        y: area.height.saturating_sub(24) / 2,
        width: 70.min(area.width.saturating_sub(4)),
        height: 24.min(area.height.saturating_sub(4)),
    };
    f.render_widget(ratatui::widgets::Clear, popup_area);
    let block = Block::bordered()
        .title(" MACHINE INFO ")
        .border_style(Style::new().fg(theme::accent()))
        .style(Style::new().bg(theme::panel()));
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(format!("  Hostname: {}", profile.identity.nickname.as_deref().unwrap_or("--")), Style::new().fg(theme::fg()))));
    lines.push(Line::from(Span::styled(format!("  Type: {:?}", profile.identity.machine_type.value), Style::new().fg(theme::fg()))));
    lines.push(Line::from(Span::raw("")));
    lines.push(Line::from(Span::styled("  Hardware", Style::new().fg(theme::accent()))));
    lines.push(Line::from(Span::styled(format!("  Extruders: {}", profile.hardware.extruders.len()), Style::new().fg(theme::fg()))));
    lines.push(Line::from(Span::styled(format!("  Hotends:   {}", profile.hardware.hotends.len()), Style::new().fg(theme::fg()))));
    lines.push(Line::from(Span::styled(format!("  MCUs:      {}", profile.hardware.mcus.len()), Style::new().fg(theme::fg()))));
    lines.push(Line::from(Span::styled(format!("  Fans:      {}", profile.hardware.cooling.len()), Style::new().fg(theme::fg()))));
    lines.push(Line::from(Span::styled(format!("  Probes:    {}", profile.hardware.probes.len()), Style::new().fg(theme::fg()))));

    if let Some(ref motion) = profile.hardware.motion_system {
        lines.push(Line::from(Span::styled(format!("  Axes:      {}", motion.axes.len()), Style::new().fg(theme::fg()))));
        if let Some(ref bv) = motion.build_volume {
            lines.push(Line::from(Span::styled(format!("  Volume:    {:.0} x {:.0} x {:.0} mm", bv.x, bv.y, bv.z), Style::new().fg(theme::fg()))));
        }
    }
    lines.push(Line::from(Span::raw("")));
    lines.push(Line::from(Span::styled("  Capabilities", Style::new().fg(theme::accent()))));

    let caps = &profile.capabilities;
    let cap_list = [
        ("Input shaping", caps.supports_input_shaping.value),
        ("Pressure advance", caps.supports_pressure_advance.value),
        ("Sensorless homing", caps.supports_sensorless_homing.value),
        ("CAN bus", caps.supports_canbus.value),
        ("BLTouch", caps.supports_bltouch.value),
        ("High temp", caps.supports_high_temperature.value),
    ];
    for &(name, supported) in &cap_list {
        let icon = if supported { "✓" } else { "—" };
        let color = if supported { theme::ok() } else { theme::muted() };
        lines.push(Line::from(Span::styled(format!("  {icon} {name}"), Style::new().fg(color))));
    }
    lines.push(Line::from(Span::raw("")));
    lines.push(Line::from(Span::styled("  Press M to close", Style::new().fg(theme::muted()))));
    f.render_widget(Paragraph::new(lines), inner);
}

fn format_duration(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{:>2}:{:>02}:{:>02}", h, m, s)
    } else {
        format!("{:>2}:{:>02}", m, s)
    }
}