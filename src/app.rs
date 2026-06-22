use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::Semaphore;

use crate::reboot::RebootConfig;
use crate::server::{Server, ServerStatus};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum OnTimeout {
    Stop,
    Continue,
}

pub struct App {
    pub servers: Vec<Server>,
    pub concurrency: usize,
    pub finished: bool,
    pub stopped: bool,
}

pub async fn run_rolling_reboot(
    servers: Arc<Mutex<Vec<Server>>>,
    config: Arc<RebootConfig>,
    concurrency: usize,
    on_timeout: OnTimeout,
) {
    let semaphore = Arc::new(Semaphore::new(concurrency));
    let stopped = Arc::new(std::sync::atomic::AtomicBool::new(false));

    let count = servers.lock().unwrap().len();
    let mut handles = Vec::with_capacity(count);

    for i in 0..count {
        let sem = semaphore.clone();
        let servers = servers.clone();
        let config = config.clone();
        let stopped = stopped.clone();

        let handle = tokio::spawn(async move {
            if stopped.load(std::sync::atomic::Ordering::Relaxed) {
                return;
            }

            let _permit = sem.acquire().await.unwrap();

            if stopped.load(std::sync::atomic::Ordering::Relaxed) {
                return;
            }

            let (host, port) = {
                let mut svrs = servers.lock().unwrap();
                svrs[i].status = ServerStatus::Rebooting;
                svrs[i].started_at = Some(Instant::now());
                (svrs[i].name.clone(), svrs[i].port)
            };

            if let Err(e) = config.reboot_server(&host, port).await {
                let mut svrs = servers.lock().unwrap();
                svrs[i].status = ServerStatus::Failed(e);
                if on_timeout == OnTimeout::Stop {
                    stopped.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                return;
            }

            // Wait a few seconds before starting verification
            tokio::time::sleep(Duration::from_secs(10)).await;

            {
                let mut svrs = servers.lock().unwrap();
                svrs[i].status = ServerStatus::Verifying;
            }

            let deadline = Instant::now() + config.timeout;
            loop {
                if Instant::now() >= deadline {
                    let mut svrs = servers.lock().unwrap();
                    svrs[i].status = ServerStatus::Failed("timeout".to_string());
                    if on_timeout == OnTimeout::Stop {
                        stopped.store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                    return;
                }

                match config.verify_server(&host, port).await {
                    Ok(()) => {
                        let mut svrs = servers.lock().unwrap();
                        svrs[i].status = ServerStatus::Success;
                        return;
                    }
                    Err(_) => {
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        let _ = handle.await;
    }
}
