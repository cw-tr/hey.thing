use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io;

pub struct ConflictResult {
    pub content: String,
    pub resolved: bool,
}

pub fn resolve_conflict_interactive(
    path: &str,
    base: &str,
    local: &str,
    remote: &str,
) -> io::Result<ConflictResult> {
    use std::io::IsTerminal;
    if !io::stdin().is_terminal() {
        return Ok(ConflictResult {
            content: local.to_string(), // Default fallback
            resolved: false,
        });
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut result = ConflictResult {
        content: local.to_string(), // Default
        resolved: false,
    };

    let res = run_app(&mut terminal, path, base, local, remote);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Ok(choice) = res {
        match choice {
            'l' => {
                result.content = local.to_string();
                result.resolved = true;
            }
            'r' => {
                result.content = remote.to_string();
                result.resolved = true;
            }
            'q' => {
                result.resolved = false;
            }
            _ => {}
        }
    }

    Ok(result)
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    path: &str,
    _base: &str,
    local: &str,
    remote: &str,
) -> io::Result<char> {
    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Header
                    Constraint::Min(10),   // Content
                    Constraint::Length(3), // Footer / Legend
                ])
                .split(f.size());

            // Header
            let header = Paragraph::new(format!(" ÇAKIŞMA ÇÖZÜCÜ: {} ", path))
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(header, chunks[0]);

            // Body (Local vs Remote)
            let body_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(chunks[1]);

            let local_view = Paragraph::new(local)
                .block(Block::default().title(" [L] Sizin Değişikliğiniz (LOCAL) ").borders(Borders::ALL))
                .wrap(Wrap { trim: true });
            f.render_widget(local_view, body_chunks[0]);

            let remote_view = Paragraph::new(remote)
                .block(Block::default().title(" [R] Uzaktaki Değişiklik (REMOTE) ").borders(Borders::ALL))
                .wrap(Wrap { trim: true });
            f.render_widget(remote_view, body_chunks[1]);

            // Footer
            let footer = Paragraph::new(" [L] Local'i Seç | [R] Remote'u Seç | [Q] İptal Et ")
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(footer, chunks[2]);
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('l') | KeyCode::Char('L') => return Ok('l'),
                KeyCode::Char('r') | KeyCode::Char('R') => return Ok('r'),
                KeyCode::Char('q') | KeyCode::Char('Q') => return Ok('q'),
                _ => {}
            }
        }
    }
}
