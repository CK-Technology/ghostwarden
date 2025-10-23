use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStatus {
    pub bridges: Vec<BridgeStatus>,
    pub nftables: Vec<NftTableStatus>,
    pub dhcp_leases: Vec<DhcpLease>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeStatus {
    pub name: String,
    pub state: String,
    pub addresses: Vec<String>,
    pub members: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NftTableStatus {
    pub name: String,
    pub family: String,
    pub chains: usize,
    pub rules: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DhcpLease {
    pub ip: String,
    pub mac: String,
    pub hostname: Option<String>,
    pub expires: Option<String>,
}

impl NetworkStatus {
    pub fn new() -> Self {
        Self {
            bridges: vec![],
            nftables: vec![],
            dhcp_leases: vec![],
        }
    }

    pub fn display(&self) {
        println!("📊 Network Status\n");

        // Display bridges
        println!("🌉 Bridges ({}):", self.bridges.len());
        if self.bridges.is_empty() {
            println!("  (none)");
        } else {
            for bridge in &self.bridges {
                println!("  • {} [{}]", bridge.name, bridge.state);
                if !bridge.addresses.is_empty() {
                    println!("    Addresses: {}", bridge.addresses.join(", "));
                }
                if !bridge.members.is_empty() {
                    println!("    Members: {}", bridge.members.join(", "));
                }
            }
        }

        // Display nftables
        println!("\n🔥 nftables ({}):", self.nftables.len());
        if self.nftables.is_empty() {
            println!("  (none)");
        } else {
            for table in &self.nftables {
                println!(
                    "  • {} ({}) - {} chains, {} rules",
                    table.name, table.family, table.chains, table.rules
                );
            }
        }

        // Display DHCP leases
        println!("\n📝 DHCP Leases ({}):", self.dhcp_leases.len());
        if self.dhcp_leases.is_empty() {
            println!("  (none)");
        } else {
            for lease in &self.dhcp_leases {
                let hostname = lease
                    .hostname
                    .as_ref()
                    .map(|h| format!(" ({})", h))
                    .unwrap_or_default();
                let expires = lease
                    .expires
                    .as_ref()
                    .map(|e| format!(" [expires: {}]", e))
                    .unwrap_or_default();
                println!("  • {}{} - {}{}", lease.ip, hostname, lease.mac, expires);
            }
        }
    }
}

impl Default for NetworkStatus {
    fn default() -> Self {
        Self::new()
    }
}
