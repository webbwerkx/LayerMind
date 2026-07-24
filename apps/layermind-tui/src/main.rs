mod app;
mod client;
mod commands;
mod layout;
mod theme;
mod widgets;

use app::AppState;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use ratatui::{Terminal, backend::CrosstermBackend};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = layermind_config::Config::load()?;
    let state = Arc::new(Mutex::new(AppState::new(config.clone())));

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    // Spawn the Moonraker polling task.
    let poll_config = config.moonraker.clone();
    let poll_state = Arc::clone(&state);
    let _poll_handle = tokio::spawn(async move {
        poll_loop(poll_config, poll_state).await;
    });

    // Main event loop.
    let res = run_loop(&mut terminal, &state).await;

    // Cleanup.
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    res
}

/// Periodic Moonraker polling loop.
async fn poll_loop(config: layermind_config::MoonrakerConfig, state: Arc<Mutex<AppState>>) {
    loop {
        // Mark as connecting on first iteration.
        {
            let mut app = state.lock().await;
            if !app.connected && !app.connecting {
                app.connecting = true;
            }
        }

        client::poll_moonraker(&config, &state).await;

        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

/// Main terminal input loop.
async fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    state: &Arc<Mutex<AppState>>,
) -> anyhow::Result<()> {
    loop {
        // Non-blocking: skip this frame if polling task holds the lock.
        if let Ok(mut guard) = state.try_lock() {
            terminal.draw(|f| {
                layout::render(f, &mut guard);
            })?;
        }

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('d') => {
                            let cmd_state = Arc::clone(state);
                            tokio::spawn(async move {
                                commands::run_diagnose(&cmd_state).await;
                            });
                        }
                        KeyCode::Char('m') => {
                            let cmd_state = Arc::clone(state);
                            tokio::spawn(async move {
                                commands::show_machine(&cmd_state).await;
                            });
                        }
                        KeyCode::Char('M') => {
                            let mut app = state.blocking_lock();
                            app.show_machine = false;
                        }
                        KeyCode::Tab => {
                            let mut app = state.blocking_lock();
                            app.focus = match app.focus {
                                app::Focus::Printer => app::Focus::Temps,
                                app::Focus::Temps => app::Focus::Events,
                                app::Focus::Events => app::Focus::Recs,
                                app::Focus::Recs => app::Focus::Printer,
                            };
                        }
                        KeyCode::Up => {
                            let mut app = state.blocking_lock();
                            if app.focus == app::Focus::Events && !app.events.is_empty() {
                                app.event_scroll = app.event_scroll.saturating_sub(1);
                            }
                        }
                        KeyCode::Down => {
                            let mut app = state.blocking_lock();
                            if app.focus == app::Focus::Events {
                                app.event_scroll = (app.event_scroll + 1).min(app.events.len().saturating_sub(1));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    Ok(())
}
