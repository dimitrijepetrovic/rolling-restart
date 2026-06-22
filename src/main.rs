mod app;
mod reboot;
mod server;
mod ui;

use std::io;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::app::{App, OnTimeout, run_rolling_reboot};
use crate::reboot::RebootConfig;
use crate::server::Server;

#[derive(Parser)]
#[command(name = "rolling-reboot", about = "Rolling reboot of servers with TUI")]
struct Cli {
    /// Comma-separated list of servers (host or host:port)
    #[arg(short, long, value_delimiter = ',')]
    servers: Vec<String>,

    /// File containing server names (one per line, or host:port per line)
    #[arg(short, long)]
    file: Option<String>,

    /// Number of servers to reboot concurrently
    #[arg(short, long, default_value = "1")]
    concurrency: usize,

    /// Timeout in minutes for each server reboot
    #[arg(short, long, default_value = "20")]
    timeout: u64,

    /// Behavior on timeout: "stop" or "continue"
    #[arg(long, default_value = "stop")]
    on_timeout: String,

    /// Custom command to verify server is back up.
    /// Use {host}, {port}, {user} as placeholders.
    #[arg(long)]
    verify_command: Option<String>,

    /// SSH username
    #[arg(short, long, default_value = "root")]
    user: String,

    /// SSH port (default for servers without explicit port)
    #[arg(short, long, default_value = "22")]
    port: u16,
}

fn parse_servers(cli: &Cli) -> Result<Vec<Server>> {
    let mut servers = Vec::new();

    for s in &cli.servers {
        let trimmed = s.trim();
        if !trimmed.is_empty() {
            servers.push(Server::parse(trimmed, cli.port));
        }
    }

    if let Some(ref path) = cli.file {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read server file: {path}"))?;
        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                servers.push(Server::parse(trimmed, cli.port));
            }
        }
    }

    anyhow::ensure!(!servers.is_empty(), "no servers specified");
    Ok(servers)
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let on_timeout = match cli.on_timeout.as_str() {
        "stop" => OnTimeout::Stop,
        "continue" => OnTimeout::Continue,
        other => anyhow::bail!("invalid --on-timeout value: {other} (expected 'stop' or 'continue')"),
    };

    let servers = parse_servers(&cli)?;

    let config = Arc::new(RebootConfig {
        user: cli.user,
        timeout: Duration::from_secs(cli.timeout * 60),
        verify_command: cli.verify_command,
    });

    let shared_servers = Arc::new(Mutex::new(servers));
    let concurrency = cli.concurrency;

    // Start the reboot process in background
    let reboot_servers = shared_servers.clone();
    let reboot_handle = tokio::spawn(async move {
        run_rolling_reboot(reboot_servers, config, concurrency, on_timeout).await;
    });

    // Set up terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_ui(&mut terminal, &shared_servers, concurrency, &reboot_handle).await;

    // Restore terminal
    disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

async fn run_ui(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    shared_servers: &Arc<Mutex<Vec<Server>>>,
    concurrency: usize,
    reboot_handle: &tokio::task::JoinHandle<()>,
) -> Result<()> {
    loop {
        let (servers_snapshot, finished, has_stopped) = {
            let svrs = shared_servers.lock().unwrap();
            let all_done = svrs.iter().all(|s| {
                matches!(
                    s.status,
                    crate::server::ServerStatus::Success | crate::server::ServerStatus::Failed(_)
                )
            });
            let any_stopped = svrs.iter().any(|s| {
                matches!(s.status, crate::server::ServerStatus::Failed(_))
            }) && svrs.iter().any(|s| {
                matches!(s.status, crate::server::ServerStatus::Pending)
            }) && reboot_handle.is_finished();
            (svrs.clone(), all_done || reboot_handle.is_finished(), any_stopped)
        };

        let app = App {
            servers: servers_snapshot,
            concurrency,
            finished: finished && !has_stopped,
            stopped: has_stopped,
        };

        terminal.draw(|frame| ui::draw(frame, &app))?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                    return Ok(());
                }
            }
        }
    }
}
