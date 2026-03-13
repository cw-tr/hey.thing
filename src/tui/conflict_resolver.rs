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

#[derive(PartialEq)]
enum Choice {
    Local,
    Remote,
    Both,
    Quit,
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
            Choice::Local => {
                result.content = local.to_string();
                result.resolved = true;
            }
            Choice::Remote => {
                result.content = remote.to_string();
                result.resolved = true;
            }
            Choice::Both => {
                let mut combined = local.to_string();
                if !combined.ends_with('\n') && !remote.is_empty() {
                    combined.push('\n');
                }
                combined.push_str(remote);
                result.content = combined;
                result.resolved = true;
            }
            Choice::Quit => {
                result.resolved = false;
            }
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
) -> io::Result<Choice> {
    let local_lines = generate_diff_text(base, local);
    let remote_lines = generate_diff_text(base, remote);
    let mut selected = 0; // 0: Local, 1: Both, 2: Remote

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

            let local_style = if selected == 0 { Style::default().fg(Color::Yellow) } else { Style::default() };
            let local_view = Paragraph::new(local_lines.clone())
                .block(Block::default()
                    .title(" [L] Sizin Değişikliğiniz (LOCAL) ")
                    .borders(Borders::ALL)
                    .border_style(local_style))
                .wrap(Wrap { trim: false });
            f.render_widget(local_view, body_chunks[0]);

            let remote_style = if selected == 2 { Style::default().fg(Color::Yellow) } else { Style::default() };
            let remote_view = Paragraph::new(remote_lines.clone())
                .block(Block::default()
                    .title(" [R] Uzaktaki Değişiklik (REMOTE) ")
                    .borders(Borders::ALL)
                    .border_style(remote_style))
                .wrap(Wrap { trim: false });
            f.render_widget(remote_view, body_chunks[1]);

            // Footer
            let footer_content = Line::from(vec![
                Span::styled(" [L] Yerel ", if selected == 0 { Style::default().bg(Color::Blue).fg(Color::White) } else { Style::default() }),
                Span::raw(" | "),
                Span::styled(" [B] İkisini de Al ", if selected == 1 { Style::default().bg(Color::Blue).fg(Color::White) } else { Style::default() }),
                Span::raw(" | "),
                Span::styled(" [R] Uzak ", if selected == 2 { Style::default().bg(Color::Blue).fg(Color::White) } else { Style::default() }),
                Span::raw(" | "),
                Span::styled(" [ENTER] Seç ", Style::default().add_modifier(ratatui::style::Modifier::BOLD)),
                Span::raw(" | "),
                Span::styled(" [Q] İptal ", Style::default()),
            ]);

            let footer = Paragraph::new(footer_content)
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(footer, chunks[2]);
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Left => { if selected > 0 { selected -= 1; } }
                KeyCode::Right => { if selected < 2 { selected += 1; } }
                KeyCode::Enter => {
                    return match selected {
                        0 => Ok(Choice::Local),
                        1 => Ok(Choice::Both),
                        _ => Ok(Choice::Remote),
                    }
                }
                KeyCode::Char('l') | KeyCode::Char('L') => return Ok(Choice::Local),
                KeyCode::Char('r') | KeyCode::Char('R') => return Ok(Choice::Remote),
                KeyCode::Char('b') | KeyCode::Char('B') => return Ok(Choice::Both),
                KeyCode::Char('q') | KeyCode::Char('Q') => return Ok(Choice::Quit),
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
