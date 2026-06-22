use std::time::Duration;
use tokio::process::Command;

pub struct RebootConfig {
    pub user: String,
    pub timeout: Duration,
    pub verify_command: Option<String>,
}

impl RebootConfig {
    fn ssh_base(&self, host: &str, port: u16) -> Command {
        let mut cmd = Command::new("ssh");
        cmd.args([
            "-o", "StrictHostKeyChecking=no",
            "-o", "ConnectTimeout=10",
            "-p", &port.to_string(),
            "-l", &self.user,
            host,
        ]);
        cmd
    }

    pub async fn reboot_server(&self, host: &str, port: u16) -> Result<(), String> {
        let mut cmd = self.ssh_base(host, port);
        cmd.arg("sudo reboot");
        let output = cmd.output().await.map_err(|e| format!("ssh failed: {e}"))?;
        // reboot command often returns non-zero or closes connection, that's expected
        if output.status.success() || output.status.code() == Some(255) {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Connection closed by remote is expected after reboot
            if stderr.contains("closed by remote") || stderr.contains("Connection reset") {
                Ok(())
            } else {
                Err(format!("reboot command failed: {stderr}"))
            }
        }
    }

    pub async fn verify_server(&self, host: &str, port: u16) -> Result<(), String> {
        if let Some(ref custom_cmd) = self.verify_command {
            let expanded = custom_cmd
                .replace("{host}", host)
                .replace("{port}", &port.to_string())
                .replace("{user}", &self.user);
            let output = Command::new("sh")
                .args(["-c", &expanded])
                .output()
                .await
                .map_err(|e| format!("verify command failed: {e}"))?;
            if output.status.success() {
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("verify failed: {stderr}"))
            }
        } else {
            let mut cmd = self.ssh_base(host, port);
            cmd.arg("uptime");
            let output = cmd.output().await.map_err(|e| format!("ssh failed: {e}"))?;
            if output.status.success() {
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(format!("verify failed: {stderr}"))
            }
        }
    }
}
