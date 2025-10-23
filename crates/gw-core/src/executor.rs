use crate::planner::Action;
use anyhow::Result;

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
            Action::CreateBridge { .. } => {
                println!("Executing: {}", action);
                // Actual execution happens in the CLI layer with real managers
                Ok(())
            }
            Action::AddAddress { .. } => {
                println!("Executing: {}", action);
                Ok(())
            }
            Action::EnableForwarding { .. } => {
                println!("Executing: {}", action);
                Ok(())
            }
            Action::CreateNftRuleset { .. } => {
                println!("Executing: {}", action);
                Ok(())
            }
            Action::StartDnsmasq { .. } => {
                println!("Executing: {}", action);
                Ok(())
            }
            Action::CreateVlan { .. } => {
                println!("Executing: {}", action);
                Ok(())
            }
            Action::AttachVlanToBridge { .. } => {
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
}

impl ExecutionContext {
    pub fn new(rollback_enabled: bool) -> Self {
        Self {
            actions_completed: vec![],
            rollback_enabled,
        }
    }

    pub fn record_action(&mut self, action: Action) {
        self.actions_completed.push(action);
    }

    pub fn get_rollback_actions(&self) -> Vec<Action> {
        // Generate rollback actions in reverse order
        self.actions_completed
            .iter()
            .rev()
            .filter_map(|_action| {
                // To rollback bridge creation, we'd need a DeleteBridge action
                // For now, just log what we'd rollback
                None
            })
            .collect()
    }
}
