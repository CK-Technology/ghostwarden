use crate::conflict::{Conflict, ConflictReport, ConflictSeverity};
use anyhow::Result;

pub struct ConflictDetector;

impl ConflictDetector {
    pub fn new() -> Self {
        Self
    }

    pub async fn detect_all(&self) -> Result<ConflictReport> {
        let mut report = ConflictReport::new();

        // Check NetworkManager
        if let Ok(Some(conflict)) = self.check_networkmanager().await {
            report.add_conflict(conflict);
        }

        // Check Docker
        if let Ok(Some(conflict)) = self.check_docker().await {
            report.add_conflict(conflict);
        }

        // Check UFW
        if let Ok(Some(conflict)) = self.check_ufw().await {
            report.add_conflict(conflict);
        }

        // Check firewalld
        if let Ok(Some(conflict)) = self.check_firewalld().await {
            report.add_conflict(conflict);
        }

        // Check iptables rules
        if let Ok(Some(conflict)) = self.check_iptables().await {
            report.add_conflict(conflict);
        }

        Ok(report)
    }

    async fn check_networkmanager(&self) -> Result<Option<Conflict>> {
        use tokio::process::Command;

        let output = Command::new("systemctl")
            .arg("is-active")
            .arg("NetworkManager")
            .output()
            .await?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.trim() == "active" {
                return Ok(Some(Conflict {
                    service: "NetworkManager".to_string(),
                    severity: ConflictSeverity::Warning,
                    description: "NetworkManager is active and may interfere with bridge management".to_string(),
                    suggestion: "Consider adding ghostwarden-managed bridges to NetworkManager's unmanaged-devices list".to_string(),
                }));
            }
        }

        Ok(None)
    }

    async fn check_docker(&self) -> Result<Option<Conflict>> {
        use tokio::process::Command;

        let output = Command::new("systemctl")
            .arg("is-active")
            .arg("docker")
            .output()
            .await?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.trim() == "active" {
                return Ok(Some(Conflict {
                    service: "Docker".to_string(),
                    severity: ConflictSeverity::Info,
                    description: "Docker is running and manages its own iptables/nftables rules".to_string(),
                    suggestion: "Ensure ghostwarden rules don't conflict with Docker's network chains".to_string(),
                }));
            }
        }

        Ok(None)
    }

    async fn check_ufw(&self) -> Result<Option<Conflict>> {
        use tokio::process::Command;

        let output = Command::new("ufw")
            .arg("status")
            .output()
            .await?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.contains("Status: active") {
                return Ok(Some(Conflict {
                    service: "UFW".to_string(),
                    severity: ConflictSeverity::Error,
                    description: "UFW is active and will conflict with ghostwarden's nftables rules".to_string(),
                    suggestion: "Disable UFW or migrate to ghostwarden's policy profiles: sudo ufw disable".to_string(),
                }));
            }
        }

        Ok(None)
    }

    async fn check_firewalld(&self) -> Result<Option<Conflict>> {
        use tokio::process::Command;

        let output = Command::new("systemctl")
            .arg("is-active")
            .arg("firewalld")
            .output()
            .await?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.trim() == "active" {
                return Ok(Some(Conflict {
                    service: "firewalld".to_string(),
                    severity: ConflictSeverity::Error,
                    description: "firewalld is active and will conflict with ghostwarden's nftables rules".to_string(),
                    suggestion: "Disable firewalld: sudo systemctl disable --now firewalld".to_string(),
                }));
            }
        }

        Ok(None)
    }

    async fn check_iptables(&self) -> Result<Option<Conflict>> {
        use tokio::process::Command;

        // Check for existing iptables rules (excluding Docker)
        let output = Command::new("iptables")
            .arg("-L")
            .arg("-n")
            .output()
            .await?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Count non-default chains/rules
            let line_count = stdout.lines().count();

            // Basic iptables setup has around 8-10 lines
            // If significantly more, there are custom rules
            if line_count > 20 {
                return Ok(Some(Conflict {
                    service: "iptables".to_string(),
                    severity: ConflictSeverity::Warning,
                    description: format!("Found {} iptables rules that may conflict with nftables", line_count).to_string(),
                    suggestion: "Review existing iptables rules and consider migrating to nftables".to_string(),
                }));
            }
        }

        Ok(None)
    }
}

impl Default for ConflictDetector {
    fn default() -> Self {
        Self::new()
    }
}
