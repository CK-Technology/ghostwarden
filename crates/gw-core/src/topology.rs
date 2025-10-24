use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Topology {
    pub version: u32,
    pub interfaces: HashMap<String, String>,
    pub networks: HashMap<String, Network>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Network {
    Routed(RoutedNetwork),
    Bridge(BridgeNetwork),
    Vxlan(VxlanNetwork),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutedNetwork {
    pub cidr: String,
    pub gw_ip: IpAddr,
    #[serde(default)]
    pub dhcp: bool,
    #[serde(default)]
    pub dns: Option<DnsConfig>,
    pub masq_out: Option<String>,
    #[serde(default)]
    pub forwards: Vec<PortForward>,
    #[serde(default)]
    pub policy_profile: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeNetwork {
    pub iface: String,
    pub vlan: Option<u16>,
    #[serde(default)]
    pub policy_profile: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VxlanNetwork {
    pub vni: u32,
    pub peers: Vec<IpAddr>,
    pub bridge: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsConfig {
    pub enabled: bool,
    #[serde(default)]
    pub zones: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortForward {
    pub public: String, // "0.0.0.0:4022/tcp"
    pub dst: String,    // "10.33.0.10:22"
}

impl Topology {
    pub fn from_yaml(yaml: &str) -> anyhow::Result<Self> {
        Ok(serde_yaml::from_str(yaml)?)
    }

    pub fn from_file(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_yaml(&content)
    }
}

impl Default for Topology {
    fn default() -> Self {
        Self {
            version: 1,
            interfaces: HashMap::new(),
            networks: HashMap::new(),
        }
    }
}
