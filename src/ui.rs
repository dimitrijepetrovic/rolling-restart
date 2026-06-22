use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, Paragraph, Row, Table};

use crate::app::App;
use crate::server::ServerStatus;

pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(10),
        Constraint::Length(3),
    ])
    .split(frame.area());

    draw_header(frame, chunks[0], app);
    draw_server_table(frame, chunks[1], app);
    draw_footer(frame, chunks[2], app);
}

fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let total = app.servers.len();
    let done = app
        .servers
        .iter()
        .filter(|s| matches!(s.status, ServerStatus::Success))
        .count();
    let failed = app
        .servers
        .iter()
        .filter(|s| matches!(s.status, ServerStatus::Failed(_)))
        .count();
    let active = app
        .servers
        .iter()
        .filter(|s| {
            matches!(
                s.status,
                ServerStatus::Rebooting | ServerStatus::Verifying
            )
        })
        .count();

    let ratio = if total > 0 {
        (done + failed) as f64 / total as f64
    } else {
        0.0
    };

    let label = format!(
        "{done}/{total} complete | {active} active | {failed} failed | concurrency: {}",
        app.concurrency
    );

    let gauge = Gauge::default()
        .block(Block::default().title(" Rolling Reboot ").borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Cyan))
        .ratio(ratio)
        .label(label);

    frame.render_widget(gauge, area);
}

fn draw_server_table(frame: &mut Frame, area: Rect, app: &App) {
    let mut sorted: Vec<_> = app.servers.iter().enumerate().collect();
    sorted.sort_by(|(_, a), (_, b)| {
        a.status
            .sort_order()
            .cmp(&b.status.sort_order())
            .then(a.name.cmp(&b.name))
    });

    let rows: Vec<Row> = sorted
        .iter()
        .map(|(_, server)| {
            let (status_text, style) = match &server.status {
                ServerStatus::Pending => ("Pending".to_string(), Style::default().fg(Color::DarkGray)),
                ServerStatus::Rebooting => {
                    let elapsed = server
                        .started_at
                        .map(|t| format!(" ({}s)", t.elapsed().as_secs()))
                        .unwrap_or_default();
                    (
                        format!("Rebooting{elapsed}"),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )
                }
                ServerStatus::Verifying => {
                    let elapsed = server
                        .started_at
                        .map(|t| format!(" ({}s)", t.elapsed().as_secs()))
                        .unwrap_or_default();
                    (
                        format!("Verifying{elapsed}"),
                        Style::default().fg(Color::Blue),
                    )
                }
                ServerStatus::Success => {
                    ("Success".to_string(), Style::default().fg(Color::Green))
                }
                ServerStatus::Failed(reason) => (
                    format!("FAILED: {reason}"),
                    Style::default()
                        .fg(Color::Red)
                        .add_modifier(Modifier::BOLD),
                ),
            };

            let port_str = if server.port != 22 {
                format!(":{}", server.port)
            } else {
                String::new()
            };

            Row::new(vec![
                format!("{}{port_str}", server.name),
                status_text,
            ])
            .style(style)
        })
        .collect();

    let widths = [Constraint::Percentage(40), Constraint::Percentage(60)];

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec!["Server", "Status"])
                .style(
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )
                .bottom_margin(1),
        )
        .block(Block::default().title(" Servers ").borders(Borders::ALL));

    frame.render_widget(table, area);
}

fn draw_footer(frame: &mut Frame, area: Rect, app: &App) {
    let status = if app.finished {
        let failed = app
            .servers
            .iter()
            .filter(|s| matches!(s.status, ServerStatus::Failed(_)))
            .count();
        if failed > 0 {
            Span::styled(
                format!(" Finished with {failed} failure(s). Press 'q' to exit. "),
                Style::default().fg(Color::Red),
            )
        } else {
            Span::styled(
                " All servers rebooted successfully. Press 'q' to exit. ",
                Style::default().fg(Color::Green),
            )
        }
    } else if app.stopped {
        Span::styled(
            " Stopped due to failure. Press 'q' to exit. ",
            Style::default().fg(Color::Red),
        )
    } else {
        Span::styled(
            " Press 'q' to abort | Rolling reboot in progress... ",
            Style::default().fg(Color::Cyan),
        )
    };

    let paragraph = Paragraph::new(Line::from(status))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(paragraph, area);
}
