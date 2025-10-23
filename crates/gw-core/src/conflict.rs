use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictReport {
    pub conflicts: Vec<Conflict>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    pub service: String,
    pub severity: ConflictSeverity,
    pub description: String,
    pub suggestion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictSeverity {
    Warning,
    Error,
    Info,
}

impl ConflictReport {
    pub fn new() -> Self {
        Self {
            conflicts: vec![],
        }
    }

    pub fn add_conflict(&mut self, conflict: Conflict) {
        self.conflicts.push(conflict);
    }

    pub fn has_errors(&self) -> bool {
        self.conflicts
            .iter()
            .any(|c| matches!(c.severity, ConflictSeverity::Error))
    }

    pub fn has_warnings(&self) -> bool {
        self.conflicts
            .iter()
            .any(|c| matches!(c.severity, ConflictSeverity::Warning))
    }

    pub fn display(&self) {
        if self.conflicts.is_empty() {
            println!("✅ No conflicts detected");
            return;
        }

        println!("⚠️  Detected {} potential conflicts:\n", self.conflicts.len());

        for (i, conflict) in self.conflicts.iter().enumerate() {
            let icon = match conflict.severity {
                ConflictSeverity::Error => "❌",
                ConflictSeverity::Warning => "⚠️ ",
                ConflictSeverity::Info => "ℹ️ ",
            };

            println!("{}. {} {} - {}", i + 1, icon, conflict.service, conflict.description);
            println!("   💡 {}\n", conflict.suggestion);
        }
    }
}

impl Default for ConflictReport {
    fn default() -> Self {
        Self::new()
    }
}
