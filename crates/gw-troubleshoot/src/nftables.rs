use crate::diagnostics::{DiagnosticLevel, DiagnosticResult};
use regex::Regex;
use std::process::Command;

/// nftables/iptables diagnostics
pub struct NftablesDiagnostics {
    nft_available: bool,
    iptables_available: bool,
}

impl NftablesDiagnostics {
    pub fn new() -> Self {
        let nft_available = Command::new("nft")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        let iptables_available = Command::new("iptables")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        Self {
            nft_available,
            iptables_available,
        }
    }

    pub async fn diagnose(&self) -> anyhow::Result<Vec<DiagnosticResult>> {
        let mut results = Vec::new();

        // Check if tools are available
        if !self.nft_available {
            results.push(
                DiagnosticResult::new(
                    DiagnosticLevel::Critical,
                    "nftables not found",
                    "The nft command is not available on this system",
                )
                .with_suggestion("Install nftables package")
                .with_command("sudo pacman -S nftables"),
            );
        }

        if self.nft_available {
            // Check nftables ruleset
            results.extend(self.check_nftables_ruleset().await?);

            // Check for NAT rules
            results.extend(self.check_nat_configuration().await?);

            // Check for conflicts
            results.extend(self.check_rule_conflicts().await?);
        }

        if self.iptables_available {
            // Check for iptables interference
            results.extend(self.check_iptables_interference().await?);
        }

        // Check kernel modules
        results.extend(self.check_kernel_modules().await?);

        Ok(results)
    }

    async fn check_nftables_ruleset(&self) -> anyhow::Result<Vec<DiagnosticResult>> {
        let mut results = Vec::new();

        let output = Command::new("nft").arg("list").arg("ruleset").output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            results.push(
                DiagnosticResult::new(
                    DiagnosticLevel::Error,
                    "Failed to list nftables ruleset",
                    format!("Error: {}", stderr),
                )
                .with_suggestion("Check if you have proper permissions")
                .with_command("sudo nft list ruleset"),
            );
            return Ok(results);
        }

        let ruleset = String::from_utf8_lossy(&output.stdout);

        // Check if ruleset is empty
        if ruleset.trim().is_empty() {
            results.push(DiagnosticResult::new(
                DiagnosticLevel::Info,
                "No nftables rules found",
                "The nftables ruleset is empty",
            ));
        } else {
            results.push(DiagnosticResult::new(
                DiagnosticLevel::Info,
                "nftables ruleset found",
                format!("Ruleset size: {} bytes", ruleset.len()),
            ));
        }

        // Check for ghostwarden tables
        if ruleset.contains("table inet gw") {
            results.push(DiagnosticResult::new(
                DiagnosticLevel::Info,
                "GhostWarden table found",
                "Found 'table inet gw' in ruleset",
            ));
        } else {
            results.push(
                DiagnosticResult::new(
                    DiagnosticLevel::Warning,
                    "GhostWarden table not found",
                    "No 'table inet gw' found - rules may not be applied",
                )
                .with_suggestion("Run 'gwarden net apply' to create tables"),
            );
        }

        Ok(results)
    }

    async fn check_nat_configuration(&self) -> anyhow::Result<Vec<DiagnosticResult>> {
        let mut results = Vec::new();

        let output = Command::new("nft")
            .arg("list")
            .arg("chain")
            .arg("inet")
            .arg("gw")
            .arg("postrouting")
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                let chain = String::from_utf8_lossy(&output.stdout);

                // Check for MASQUERADE rules
                if chain.contains("masquerade") {
                    results.push(DiagnosticResult::new(
                        DiagnosticLevel::Info,
                        "NAT masquerade found",
                        "MASQUERADE rule is configured",
                    ));
                } else {
                    results.push(
                        DiagnosticResult::new(
                            DiagnosticLevel::Warning,
                            "No masquerade rule found",
                            "NAT masquerading may not be working",
                        )
                        .with_suggestion("Check your topology YAML for masq_out configuration"),
                    );
                }

                // Check output interface
                let iface_regex = Regex::new(r#"oifname\s+"([^"]+)""#).unwrap();
                if let Some(cap) = iface_regex.captures(&chain) {
                    let iface = &cap[1];
                    results.push(DiagnosticResult::new(
                        DiagnosticLevel::Info,
                        "NAT output interface",
                        format!("Masquerading via interface: {}", iface),
                    ));
                }
            }
        }

        Ok(results)
    }

    async fn check_rule_conflicts(&self) -> anyhow::Result<Vec<DiagnosticResult>> {
        let mut results = Vec::new();

        // Check for duplicate chains
        let output = Command::new("nft").arg("list").arg("ruleset").output()?;

        if output.status.success() {
            let ruleset = String::from_utf8_lossy(&output.stdout);

            // Count postrouting chains
            let postrouting_count = ruleset.matches("chain postrouting").count();
            if postrouting_count > 2 {
                results.push(
                    DiagnosticResult::new(
                        DiagnosticLevel::Warning,
                        "Multiple postrouting chains detected",
                        format!(
                            "Found {} postrouting chains - may cause conflicts",
                            postrouting_count
                        ),
                    )
                    .with_suggestion("Review your nftables configuration for conflicts")
                    .with_command("nft list ruleset | grep -A5 'chain postrouting'"),
                );
            }

            // Check for policy drops that might block traffic
            if ruleset.contains("policy drop") {
                results.push(
                    DiagnosticResult::new(
                        DiagnosticLevel::Warning,
                        "Drop policy detected",
                        "Found 'policy drop' - ensure explicit accept rules exist",
                    )
                    .with_suggestion("Verify that necessary traffic is explicitly allowed"),
                );
            }
        }

        Ok(results)
    }

    async fn check_iptables_interference(&self) -> anyhow::Result<Vec<DiagnosticResult>> {
        let mut results = Vec::new();

        // Check if iptables has rules that might conflict
        let output = Command::new("iptables")
            .arg("-t")
            .arg("nat")
            .arg("-L")
            .arg("-n")
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                let rules = String::from_utf8_lossy(&output.stdout);

                // Look for non-empty chains
                let has_rules = rules
                    .lines()
                    .filter(|line| !line.starts_with("Chain") && !line.starts_with("target"))
                    .any(|line| !line.trim().is_empty());

                if has_rules {
                    results.push(
                        DiagnosticResult::new(
                            DiagnosticLevel::Warning,
                            "iptables NAT rules detected",
                            "Found iptables NAT rules that may conflict with nftables"
                        )
                        .with_suggestion("Consider migrating iptables rules to nftables or ensuring they don't conflict")
                        .with_command("iptables -t nat -L -n -v")
                    );
                }
            }
        }

        // Check iptables filter table
        let output = Command::new("iptables").arg("-L").arg("-n").output();

        if let Ok(output) = output {
            if output.status.success() {
                let rules = String::from_utf8_lossy(&output.stdout);

                if rules.contains("DOCKER") || rules.contains("docker") {
                    results.push(
                        DiagnosticResult::new(
                            DiagnosticLevel::Info,
                            "Docker iptables rules found",
                            "Docker is managing iptables rules",
                        )
                        .with_suggestion(
                            "Docker and nftables can coexist, but be aware of rule precedence",
                        ),
                    );
                }
            }
        }

        Ok(results)
    }

    async fn check_kernel_modules(&self) -> anyhow::Result<Vec<DiagnosticResult>> {
        let mut results = Vec::new();

        let required_modules = vec![
            ("nf_tables", "Core nftables support"),
            ("nf_nat", "NAT support"),
            ("nf_conntrack", "Connection tracking"),
            ("br_netfilter", "Bridge netfilter support"),
        ];

        for (module, description) in required_modules {
            let output = Command::new("lsmod").output()?;

            if output.status.success() {
                let modules = String::from_utf8_lossy(&output.stdout);

                if modules.contains(module) {
                    results.push(DiagnosticResult::new(
                        DiagnosticLevel::Info,
                        format!("Kernel module: {}", module),
                        format!("{} is loaded", description),
                    ));
                } else {
                    results.push(
                        DiagnosticResult::new(
                            DiagnosticLevel::Warning,
                            format!("Missing kernel module: {}", module),
                            format!("{} is not loaded", description),
                        )
                        .with_suggestion(format!("Load the module with: modprobe {}", module))
                        .with_command(format!("sudo modprobe {}", module)),
                    );
                }
            }
        }

        // Check sysctl settings for forwarding
        let output = Command::new("sysctl").arg("net.ipv4.ip_forward").output();

        if let Ok(output) = output {
            if output.status.success() {
                let value = String::from_utf8_lossy(&output.stdout);
                if value.contains("= 1") {
                    results.push(DiagnosticResult::new(
                        DiagnosticLevel::Info,
                        "IP forwarding enabled",
                        "net.ipv4.ip_forward = 1",
                    ));
                } else {
                    results.push(
                        DiagnosticResult::new(
                            DiagnosticLevel::Error,
                            "IP forwarding disabled",
                            "net.ipv4.ip_forward = 0 - routing will not work",
                        )
                        .with_suggestion("Enable IP forwarding for NAT and routing to work")
                        .with_command("sudo sysctl -w net.ipv4.ip_forward=1"),
                    );
                }
            }
        }

        // Check bridge netfilter settings
        let output = Command::new("sysctl")
            .arg("net.bridge.bridge-nf-call-iptables")
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                let value = String::from_utf8_lossy(&output.stdout);
                results.push(DiagnosticResult::new(
                    DiagnosticLevel::Info,
                    "Bridge netfilter setting",
                    value.trim().to_string(),
                ));
            }
        }

        Ok(results)
    }
}

impl Default for NftablesDiagnostics {
    fn default() -> Self {
        Self::new()
    }
}
