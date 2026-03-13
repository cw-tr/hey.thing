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
use ratatui::text::{Line, Span};
use ratatui::style::{Color, Style};
use similar::{Algorithm, ChangeTag, TextDiff};
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
    base: &str,
    local: &str,
    remote: &str,
) -> io::Result<char> {
    let local_lines = generate_diff_text(base, local);
    let remote_lines = generate_diff_text(base, remote);

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

            let local_view = Paragraph::new(local_lines.clone())
                .block(Block::default().title(" [L] Sizin Değişikliğiniz (LOCAL) ").borders(Borders::ALL))
                .wrap(Wrap { trim: false });
            f.render_widget(local_view, body_chunks[0]);

            let remote_view = Paragraph::new(remote_lines.clone())
                .block(Block::default().title(" [R] Uzaktaki Değişiklik (REMOTE) ").borders(Borders::ALL))
                .wrap(Wrap { trim: false });
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

fn generate_diff_text<'a>(base: &'a str, target: &'a str) -> Vec<Line<'a>> {
    let diff = TextDiff::configure()
        .algorithm(Algorithm::Myers)
        .diff_lines(base, target);

    let mut lines = Vec::new();
    for change in diff.iter_all_changes() {
        let (sign, style) = match change.tag() {
            ChangeTag::Delete => ("- ", Style::default().fg(Color::Red)),
            ChangeTag::Insert => ("+ ", Style::default().fg(Color::Green)),
            ChangeTag::Equal => ("  ", Style::default().fg(Color::Gray)),
        };
        
        lines.push(Line::from(vec![
            Span::styled(sign, style),
            Span::styled(change.value().trim_end(), style),
        ]));
    }
    
    if lines.is_empty() && !target.is_empty() {
        for line in target.lines() {
            lines.push(Line::from(vec![Span::raw(line)]));
        }
    }

    lines
}
