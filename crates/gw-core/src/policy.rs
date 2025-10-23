use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyProfile {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub allowed_ingress_cidrs: Vec<String>,
    #[serde(default)]
    pub allowed_egress_cidrs: Vec<String>,
    #[serde(default)]
    pub services: Vec<Service>,
    #[serde(default = "default_drop_policy")]
    pub default_action: Action,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    pub protocol: Protocol,
    pub port: u16,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Tcp,
    Udp,
    Icmp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Accept,
    Drop,
    Reject,
}

fn default_drop_policy() -> Action {
    Action::Drop
}
