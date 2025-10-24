use crate::planner::{Action, Plan};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::{sleep, timeout};

pub const ROLLBACK_FILENAME: &str = "rollback.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackRecord {
    pub created_at: u64,
    pub plan: Option<Plan>,
    pub actions: Vec<Action>,
    pub nft_snapshots: HashMap<String, Option<String>>,
}

impl RollbackRecord {
    pub fn new(
        plan: Option<Plan>,
        actions: Vec<Action>,
        nft_snapshots: HashMap<String, Option<String>>,
    ) -> Self {
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| Duration::from_secs(0))
            .as_secs();

        Self {
            created_at,
            plan,
            actions,
            nft_snapshots,
        }
    }
}

pub fn default_state_dir() -> Result<PathBuf> {
    if let Ok(path) = std::env::var("XDG_STATE_HOME") {
        return Ok(PathBuf::from(path).join("gwarden"));
    }

    let home = std::env::var("HOME").context("HOME environment variable not set")?;
    Ok(PathBuf::from(home).join(".local/state/gwarden"))
}

pub fn default_record_path() -> Result<PathBuf> {
    Ok(default_state_dir()?.join(ROLLBACK_FILENAME))
}

pub fn save_record(record: &RollbackRecord) -> Result<PathBuf> {
    let path = default_record_path()?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }

    let data = serde_json::to_vec_pretty(record)?;
    fs::write(&path, data)?;
    Ok(path)
}

pub fn load_record() -> Result<Option<RollbackRecord>> {
    let path = default_record_path()?;
    if !path.exists() {
        return Ok(None);
    }

    let data = fs::read(&path)?;
    let record = serde_json::from_slice(&data)?;
    Ok(Some(record))
}

pub fn clear_record() -> Result<()> {
    let path = default_record_path()?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub struct RollbackManager {
    pub timeout_seconds: u64,
    pub ssh_check_enabled: bool,
}

impl RollbackManager {
    pub fn new(timeout_seconds: u64) -> Self {
        Self {
            timeout_seconds,
            ssh_check_enabled: true,
        }
    }

    /// Wait for user confirmation or timeout
    /// If timeout expires without confirmation, trigger rollback
    pub async fn wait_for_confirmation(&self) -> Result<bool> {
        println!("\n⏰ Auto-rollback armed for {}s", self.timeout_seconds);
        println!("   Press ENTER to confirm changes, or wait for auto-rollback...");

        // Create a channel for user input
        let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(1);

        // Spawn task to read from stdin
        tokio::task::spawn_blocking(move || {
            use std::io::{BufRead, stdin};
            let stdin = stdin();
            let mut lines = stdin.lock().lines();
            if lines.next().is_some() {
                let _ = tx.blocking_send(());
            }
        });

        // Wait for either user input or timeout
        match timeout(Duration::from_secs(self.timeout_seconds), rx.recv()).await {
            Ok(Some(_)) => {
                println!("✅ Changes confirmed!");
                Ok(true)
            }
            Ok(None) | Err(_) => {
                println!("\n⚠️  Timeout reached! Rolling back changes...");
                Ok(false)
            }
        }
    }

    /// Check if SSH is still accessible by testing TCP connection to localhost:22
    pub async fn check_ssh_connectivity(&self) -> Result<bool> {
        if !self.ssh_check_enabled {
            return Ok(true);
        }

        // Try to connect to localhost SSH port
        let addr = "127.0.0.1:22";
        let timeout_duration = Duration::from_secs(3);

        match timeout(timeout_duration, tokio::net::TcpStream::connect(addr)).await {
            Ok(Ok(_stream)) => {
                // Connection succeeded
                Ok(true)
            }
            Ok(Err(_)) | Err(_) => {
                // Connection failed or timed out
                Ok(false)
            }
        }
    }

    /// Check if a custom host:port is accessible (for monitoring other hosts)
    pub async fn check_tcp_connectivity(&self, addr: &str, timeout_secs: u64) -> Result<bool> {
        let timeout_duration = Duration::from_secs(timeout_secs);

        match timeout(timeout_duration, tokio::net::TcpStream::connect(addr)).await {
            Ok(Ok(_stream)) => Ok(true),
            Ok(Err(_)) | Err(_) => Ok(false),
        }
    }

    /// Monitor SSH connectivity and trigger rollback if lost
    pub async fn monitor_ssh_with_rollback<F, Fut>(&self, rollback_fn: F) -> Result<()>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        let check_interval = Duration::from_secs(5);
        let max_checks = self.timeout_seconds / 5;

        for i in 0..max_checks {
            sleep(check_interval).await;

            if !self.check_ssh_connectivity().await? {
                println!("\n❌ SSH connection lost! Rolling back...");
                rollback_fn().await?;
                return Ok(());
            }

            if i % 2 == 0 {
                println!(
                    "⏳ Waiting for confirmation... ({}s remaining)",
                    self.timeout_seconds - (i * 5)
                );
            }
        }

        println!("\n⚠️  Timeout expired without confirmation. Rolling back...");
        rollback_fn().await?;

        Ok(())
    }
}

/// Rollback state for tracking what needs to be undone
pub struct RollbackState {
    pub bridges_created: Vec<String>,
    pub addresses_added: Vec<(String, String)>,
    pub nft_tables_created: Vec<String>,
    pub dnsmasq_configs_written: Vec<String>,
}

impl RollbackState {
    pub fn new() -> Self {
        Self {
            bridges_created: vec![],
            addresses_added: vec![],
            nft_tables_created: vec![],
            dnsmasq_configs_written: vec![],
        }
    }

    /// Print what will be rolled back (actual execution happens in CLI)
    pub fn display(&self) {
        println!("🔄 Rolling back changes...");

        if !self.dnsmasq_configs_written.is_empty() {
            println!("  Will stop dnsmasq and delete configs:");
            for config in &self.dnsmasq_configs_written {
                println!("    - {}", config);
            }
        }

        if !self.nft_tables_created.is_empty() {
            println!("  Will delete nftables tables:");
            for table in &self.nft_tables_created {
                println!("    - {}", table);
            }
        }

        if !self.addresses_added.is_empty() {
            println!("  Will remove addresses:");
            for (iface, addr) in &self.addresses_added {
                println!("    - {} from {}", addr, iface);
            }
        }

        if !self.bridges_created.is_empty() {
            println!("  Will delete bridges:");
            for bridge in &self.bridges_created {
                println!("    - {}", bridge);
            }
        }
    }
}

impl Default for RollbackState {
    fn default() -> Self {
        Self::new()
    }
}
