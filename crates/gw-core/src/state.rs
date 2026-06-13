use crate::planner::{Action, Plan};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const APPLY_STATE_FILENAME: &str = "applied-state.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApplyState {
    pub transaction_id: String,
    pub created_at: u64,
    pub plan: Plan,
    pub owned_resources: Vec<OwnedResource>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OwnedResource {
    Bridge { name: String },
    Address { iface: String, addr: String },
    NftTable { table: String },
    DnsmasqConfig { path: String },
    Vlan { name: String },
}

impl ApplyState {
    pub fn from_plan(transaction_id: String, plan: Plan, completed_actions: &[Action]) -> Self {
        Self {
            transaction_id,
            created_at: unix_timestamp(),
            plan,
            owned_resources: owned_resources_from_actions(completed_actions),
        }
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create state directory {}", parent.display())
            })?;
        }

        let data = serde_json::to_vec_pretty(self)?;
        fs::write(path, data).with_context(|| format!("Failed to write {}", path.display()))?;
        Ok(())
    }

    pub fn load_from(path: &Path) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }

        let data = fs::read(path).with_context(|| format!("Failed to read {}", path.display()))?;
        Ok(Some(serde_json::from_slice(&data)?))
    }
}

pub fn default_apply_state_path() -> Result<PathBuf> {
    Ok(crate::rollback::default_state_dir()?.join(APPLY_STATE_FILENAME))
}

pub fn new_transaction_id() -> String {
    format!("gw-{}", unix_timestamp())
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs()
}

fn owned_resources_from_actions(actions: &[Action]) -> Vec<OwnedResource> {
    actions
        .iter()
        .filter_map(|action| match action {
            Action::CreateBridge { name, .. } => Some(OwnedResource::Bridge { name: name.clone() }),
            Action::AddAddress { iface, addr } => Some(OwnedResource::Address {
                iface: iface.clone(),
                addr: addr.clone(),
            }),
            Action::CreateNftRuleset { table, .. } => Some(OwnedResource::NftTable {
                table: table.clone(),
            }),
            Action::StartDnsmasq { config_path } => Some(OwnedResource::DnsmasqConfig {
                path: config_path.clone(),
            }),
            Action::CreateVlan { name, .. } => Some(OwnedResource::Vlan { name: name.clone() }),
            Action::EnableForwarding { .. } | Action::AttachVlanToBridge { .. } => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_state_tracks_owned_resources() {
        let plan = Plan {
            actions: vec![
                Action::CreateBridge {
                    name: "br-test".into(),
                    cidr: None,
                },
                Action::AddAddress {
                    iface: "br-test".into(),
                    addr: "10.10.0.1/24".into(),
                },
                Action::EnableForwarding {
                    iface: "br-test".into(),
                },
            ],
        };

        let state = ApplyState::from_plan("gw-test".into(), plan.clone(), &plan.actions);

        assert_eq!(state.transaction_id, "gw-test");
        assert_eq!(
            state.owned_resources,
            vec![
                OwnedResource::Bridge {
                    name: "br-test".into()
                },
                OwnedResource::Address {
                    iface: "br-test".into(),
                    addr: "10.10.0.1/24".into()
                }
            ]
        );
    }

    #[test]
    fn apply_state_json_round_trip() {
        let plan = Plan {
            actions: vec![Action::CreateNftRuleset {
                table: "gw-test".into(),
                policy_profile: Some("routed-tight".into()),
            }],
        };
        let state = ApplyState::from_plan("gw-test".into(), plan.clone(), &plan.actions);

        let json = serde_json::to_string(&state).unwrap();
        let decoded: ApplyState = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded, state);
    }
}
