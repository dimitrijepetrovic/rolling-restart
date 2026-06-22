use std::fmt;
use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerStatus {
    Pending,
    Rebooting,
    Verifying,
    Success,
    Failed(String),
}

impl ServerStatus {
    pub fn sort_order(&self) -> u8 {
        match self {
            ServerStatus::Failed(_) => 0,
            ServerStatus::Rebooting => 1,
            ServerStatus::Verifying => 2,
            ServerStatus::Success => 3,
            ServerStatus::Pending => 4,
        }
    }
}

impl fmt::Display for ServerStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServerStatus::Pending => write!(f, "Pending"),
            ServerStatus::Rebooting => write!(f, "Rebooting"),
            ServerStatus::Verifying => write!(f, "Verifying"),
            ServerStatus::Success => write!(f, "Success"),
            ServerStatus::Failed(reason) => write!(f, "Failed: {reason}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Server {
    pub name: String,
    pub port: u16,
    pub status: ServerStatus,
    pub started_at: Option<Instant>,
}

impl Server {
    pub fn new(name: String, port: u16) -> Self {
        Self {
            name,
            port,
            status: ServerStatus::Pending,
            started_at: None,
        }
    }

    pub fn parse(input: &str, default_port: u16) -> Self {
        let trimmed = input.trim();
        if let Some((host, port_str)) = trimmed.rsplit_once(':') {
            if let Ok(port) = port_str.parse::<u16>() {
                return Self::new(host.to_string(), port);
            }
        }
        Self::new(trimmed.to_string(), default_port)
    }
}
