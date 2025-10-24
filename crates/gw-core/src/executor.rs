use crate::planner::{Action, Plan};
use crate::rollback::RollbackRecord;
use anyhow::Result;
use std::collections::HashMap;

/// Executor trait for applying network changes
pub struct Executor {
    dry_run: bool,
}

impl Executor {
    pub fn new(dry_run: bool) -> Self {
        Self { dry_run }
    }

    pub async fn execute_action(&self, action: &Action) -> Result<()> {
        if self.dry_run {
            println!("[DRY RUN] Would execute: {}", action);
            return Ok(());
        }

        match action {
            Action::CreateBridge { .. }
            | Action::AddAddress { .. }
            | Action::EnableForwarding { .. }
            | Action::CreateNftRuleset { .. }
            | Action::StartDnsmasq { .. }
            | Action::CreateVlan { .. }
            | Action::AttachVlanToBridge { .. } => {
                println!("Executing: {}", action);
                Ok(())
            }
        }
    }
}

/// Plan execution context with rollback state
pub struct ExecutionContext {
    pub actions_completed: Vec<Action>,
    pub rollback_enabled: bool,
    pub nft_snapshots: HashMap<String, Option<String>>,
    pub plan: Option<Plan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RollbackOp {
    DeleteBridge {
        name: String,
    },
    RemoveAddress {
        iface: String,
        addr: String,
    },
    RestoreNft {
        table: String,
        snapshot: Option<String>,
    },
    DeleteDnsmasqConfig {
        path: String,
    },
    DeleteVlan {
        name: String,
    },
}

impl ExecutionContext {
    pub fn new(rollback_enabled: bool) -> Self {
        Self {
            actions_completed: vec![],
            rollback_enabled,
            nft_snapshots: HashMap::new(),
            plan: None,
        }
    }

    pub fn record_action(&mut self, action: Action) {
        self.actions_completed.push(action);
    }

    pub fn attach_plan(&mut self, plan: Plan) {
        self.plan = Some(plan);
    }

    pub fn record_nft_snapshot(&mut self, table: String, snapshot: Option<String>) {
        self.nft_snapshots.insert(table, snapshot);
    }

    pub fn nft_snapshot(&self, table: &str) -> Option<&Option<String>> {
        self.nft_snapshots.get(table)
    }

    pub fn to_rollback_record(&self) -> RollbackRecord {
        RollbackRecord::new(
            self.plan.clone(),
            self.actions_completed.clone(),
            self.nft_snapshots.clone(),
        )
    }

    pub fn from_rollback_record(record: RollbackRecord) -> Self {
        Self {
            actions_completed: record.actions,
            rollback_enabled: true,
            nft_snapshots: record.nft_snapshots,
            plan: record.plan,
        }
    }

    pub fn rollback_operations(&self) -> Vec<RollbackOp> {
        let mut ops = Vec::new();

        for action in self.actions_completed.iter().rev() {
            match action {
                Action::CreateBridge { name, .. } => {
                    ops.push(RollbackOp::DeleteBridge { name: name.clone() });
                }
                Action::AddAddress { iface, addr } => {
                    ops.push(RollbackOp::RemoveAddress {
                        iface: iface.clone(),
                        addr: addr.clone(),
                    });
                }
                Action::CreateNftRuleset { table, .. } => {
                    let snapshot = self.nft_snapshot(table).cloned().unwrap_or(None);
                    ops.push(RollbackOp::RestoreNft {
                        table: table.clone(),
                        snapshot,
                    });
                }
                Action::StartDnsmasq { config_path } => {
                    ops.push(RollbackOp::DeleteDnsmasqConfig {
                        path: config_path.clone(),
                    });
                }
                Action::CreateVlan { name, .. } => {
                    ops.push(RollbackOp::DeleteVlan { name: name.clone() });
                }
                Action::EnableForwarding { .. } | Action::AttachVlanToBridge { .. } => {
                    // No direct rollback operation or handled elsewhere
                }
            }
        }

        ops
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rollback_operations_include_snapshot() {
        let mut ctx = ExecutionContext::new(true);
        ctx.record_action(Action::CreateNftRuleset {
            table: "gw-nat".into(),
            policy_profile: Some("routed-tight".into()),
        });
        ctx.record_nft_snapshot("gw-nat".into(), Some("snapshot".into()));

        let ops = ctx.rollback_operations();
        assert_eq!(ops.len(), 1);
        match &ops[0] {
            RollbackOp::RestoreNft { table, snapshot } => {
                assert_eq!(table, "gw-nat");
                assert_eq!(snapshot.as_deref(), Some("snapshot"));
            }
            other => panic!("unexpected op: {:?}", other),
        }
    }

    #[test]
    fn rollback_operations_handle_missing_snapshot() {
        let mut ctx = ExecutionContext::new(true);
        ctx.record_action(Action::CreateNftRuleset {
            table: "gw-nat".into(),
            policy_profile: None,
        });

        let ops = ctx.rollback_operations();
        assert_eq!(ops.len(), 1);
        match &ops[0] {
            RollbackOp::RestoreNft { table, snapshot } => {
                assert_eq!(table, "gw-nat");
                assert!(snapshot.is_none());
            }
            other => panic!("unexpected op: {:?}", other),
        }
    }
}
