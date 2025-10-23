use serde::{Deserialize, Serialize};
use std::fmt;

/// Severity level for diagnostic findings
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DiagnosticLevel {
    /// Informational - no action needed
    Info,
    /// Warning - potential issue
    Warning,
    /// Error - requires attention
    Error,
    /// Critical - blocking issue
    Critical,
}

impl fmt::Display for DiagnosticLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (symbol, name) = match self {
            DiagnosticLevel::Info => ("ℹ️", "INFO"),
            DiagnosticLevel::Warning => ("⚠️", "WARN"),
            DiagnosticLevel::Error => ("❌", "ERROR"),
            DiagnosticLevel::Critical => ("🔥", "CRITICAL"),
        };
        write!(f, "{} {}", symbol, name)
    }
}

/// A single diagnostic result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticResult {
    pub level: DiagnosticLevel,
    pub title: String,
    pub details: String,
    pub suggestion: Option<String>,
    pub command: Option<String>,
}

impl DiagnosticResult {
    pub fn new(level: DiagnosticLevel, title: impl Into<String>, details: impl Into<String>) -> Self {
        Self {
            level,
            title: title.into(),
            details: details.into(),
            suggestion: None,
            command: None,
        }
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    pub fn with_command(mut self, command: impl Into<String>) -> Self {
        self.command = Some(command.into());
        self
    }

    pub fn display(&self) {
        println!("\n{} {}", self.level, self.title);
        println!("  {}", self.details);

        if let Some(suggestion) = &self.suggestion {
            println!("  💡 Suggestion: {}", suggestion);
        }

        if let Some(command) = &self.command {
            println!("  🔧 Fix: {}", command);
        }
    }
}

/// Complete diagnostic report
#[derive(Debug, Default)]
pub struct DiagnosticReport {
    sections: Vec<(String, Vec<DiagnosticResult>)>,
}

impl DiagnosticReport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_section(&mut self, name: impl Into<String>, results: Vec<DiagnosticResult>) {
        self.sections.push((name.into(), results));
    }

    pub fn has_errors(&self) -> bool {
        self.sections.iter().any(|(_, results)| {
            results.iter().any(|r| r.level >= DiagnosticLevel::Error)
        })
    }

    pub fn has_warnings(&self) -> bool {
        self.sections.iter().any(|(_, results)| {
            results.iter().any(|r| r.level == DiagnosticLevel::Warning)
        })
    }

    pub fn display(&self) {
        println!("\n╔═══════════════════════════════════════════════════════════════╗");
        println!("║            GhostWarden Troubleshooting Report                ║");
        println!("╚═══════════════════════════════════════════════════════════════╝");

        for (section_name, results) in &self.sections {
            if results.is_empty() {
                continue;
            }

            println!("\n━━━ {} ━━━", section_name);

            for result in results {
                result.display();
            }
        }

        // Summary
        let total_errors = self.count_by_level(DiagnosticLevel::Error) +
                          self.count_by_level(DiagnosticLevel::Critical);
        let total_warnings = self.count_by_level(DiagnosticLevel::Warning);

        println!("\n━━━ Summary ━━━");
        if total_errors > 0 {
            println!("  ❌ {} error(s) found", total_errors);
        }
        if total_warnings > 0 {
            println!("  ⚠️  {} warning(s) found", total_warnings);
        }
        if total_errors == 0 && total_warnings == 0 {
            println!("  ✅ All checks passed!");
        }
    }

    fn count_by_level(&self, level: DiagnosticLevel) -> usize {
        self.sections.iter()
            .flat_map(|(_, results)| results.iter())
            .filter(|r| r.level == level)
            .count()
    }
}
