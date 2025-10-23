use anyhow::{Context, Result};
use std::path::Path;

pub struct DnsmasqManager;

impl DnsmasqManager {
    pub fn new() -> Self {
        Self
    }

    pub fn generate_config(
        &self,
        bridge: &str,
        cidr: &str,
        zones: &[String],
    ) -> Result<String> {
        // Parse CIDR to get network range for DHCP
        let (network, prefix) = parse_cidr(cidr)?;
        let dhcp_range = calculate_dhcp_range(&network, prefix)?;

        let mut config = String::new();

        // Bind to specific interface
        config.push_str(&format!("# Ghostwarden configuration for {}\n", bridge));
        config.push_str(&format!("interface={}\n", bridge));
        config.push_str("bind-interfaces\n");
        config.push_str("except-interface=lo\n\n");

        // DHCP configuration
        config.push_str(&format!("# DHCP range for {}\n", bridge));
        config.push_str(&format!("dhcp-range={}\n", dhcp_range));
        config.push_str(&format!("dhcp-option=option:router,{}\n", network));
        config.push_str(&format!("dhcp-option=option:dns-server,{}\n\n", network));

        // DNS configuration
        if !zones.is_empty() {
            config.push_str("# Local DNS zones\n");
            for zone in zones {
                config.push_str(&format!("local=/{}/\n", zone));
                config.push_str(&format!("domain={}\n", zone));
            }
            config.push_str("\n");
        }

        // Additional settings
        config.push_str("# Additional settings\n");
        config.push_str("dhcp-authoritative\n");
        config.push_str("dhcp-leasefile=/var/lib/misc/dnsmasq.leases\n");
        config.push_str(&format!("log-facility=/var/log/dnsmasq-{}.log\n", bridge));

        Ok(config)
    }

    pub fn write_config(&self, path: &str, content: &str) -> Result<()> {
        let config_path = Path::new(path);

        // Create parent directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)
                .context(format!("Failed to create directory {:?}", parent))?;
        }

        // Write config file
        std::fs::write(config_path, content)
            .context(format!("Failed to write config to {}", path))?;

        println!("Wrote dnsmasq config to {}", path);
        Ok(())
    }

    pub async fn restart(&self) -> Result<()> {
        use tokio::process::Command;

        // Try systemctl restart dnsmasq
        let output = Command::new("systemctl")
            .arg("restart")
            .arg("dnsmasq")
            .output()
            .await
            .context("Failed to run systemctl restart dnsmasq")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to restart dnsmasq: {}", stderr);
        }

        println!("Restarted dnsmasq service");
        Ok(())
    }

    pub async fn enable(&self) -> Result<()> {
        use tokio::process::Command;

        // Enable dnsmasq service
        let output = Command::new("systemctl")
            .arg("enable")
            .arg("dnsmasq")
            .output()
            .await
            .context("Failed to run systemctl enable dnsmasq")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to enable dnsmasq: {}", stderr);
        }

        println!("Enabled dnsmasq service");
        Ok(())
    }

    /// Delete a dnsmasq config file
    pub fn delete_config(&self, path: &str) -> Result<()> {
        if std::path::Path::new(path).exists() {
            std::fs::remove_file(path)
                .context(format!("Failed to delete config file: {}", path))?;
            println!("Deleted dnsmasq config: {}", path);
        }
        Ok(())
    }
}

impl Default for DnsmasqManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse CIDR notation: "10.33.0.0/24" -> ("10.33.0.0", 24)
fn parse_cidr(cidr: &str) -> Result<(String, u8)> {
    let parts: Vec<&str> = cidr.split('/').collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid CIDR format: {}", cidr);
    }

    let network = parts[0].to_string();
    let prefix: u8 = parts[1]
        .parse()
        .context(format!("Invalid prefix length: {}", parts[1]))?;

    Ok((network, prefix))
}

/// Calculate DHCP range from network CIDR
/// For 10.33.0.0/24, returns "10.33.0.10,10.33.0.250,12h"
fn calculate_dhcp_range(network: &str, _prefix: u8) -> Result<String> {
    use std::net::Ipv4Addr;

    let ip: Ipv4Addr = network
        .parse()
        .context(format!("Invalid IP address: {}", network))?;

    let octets = ip.octets();

    // Simple range: .10 to .250 for /24 networks
    // For other sizes, you'd need more sophisticated calculation
    let start_ip = format!("{}.{}.{}.10", octets[0], octets[1], octets[2]);
    let end_ip = format!("{}.{}.{}.250", octets[0], octets[1], octets[2]);

    Ok(format!("{},{},12h", start_ip, end_ip))
}
