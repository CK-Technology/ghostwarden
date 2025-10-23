use anyhow::Result;
use gw_core::DhcpLease;
use std::path::Path;

pub struct LeaseReader;

impl LeaseReader {
    pub fn new() -> Self {
        Self
    }

    pub fn read_leases(&self, lease_file: &str) -> Result<Vec<DhcpLease>> {
        let path = Path::new(lease_file);
        if !path.exists() {
            return Ok(vec![]);
        }

        let content = std::fs::read_to_string(path)?;
        let mut leases = vec![];

        for line in content.lines() {
            // dnsmasq lease format: timestamp mac ip hostname client-id
            // Example: 1234567890 aa:bb:cc:dd:ee:ff 10.33.0.100 myhost *
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let timestamp = parts[0];
                let mac = parts[1].to_string();
                let ip = parts[2].to_string();
                let hostname = if parts[3] != "*" {
                    Some(parts[3].to_string())
                } else {
                    None
                };

                // Convert timestamp to expiry time
                let expires = if let Ok(ts) = timestamp.parse::<i64>() {
                    use std::time::{SystemTime, UNIX_EPOCH};
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as i64;

                    if ts > now {
                        let remaining = ts - now;
                        Some(format!("{}s", remaining))
                    } else {
                        Some("expired".to_string())
                    }
                } else {
                    None
                };

                leases.push(DhcpLease {
                    ip,
                    mac,
                    hostname,
                    expires,
                });
            }
        }

        Ok(leases)
    }

    pub fn read_default_leases(&self) -> Result<Vec<DhcpLease>> {
        self.read_leases("/var/lib/misc/dnsmasq.leases")
    }
}

impl Default for LeaseReader {
    fn default() -> Self {
        Self::new()
    }
}
