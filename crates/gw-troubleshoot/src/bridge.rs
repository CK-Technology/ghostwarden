use crate::diagnostics::{DiagnosticLevel, DiagnosticResult};
use regex::Regex;
use std::process::Command;

/// Bridge networking diagnostics
pub struct BridgeDiagnostics;

impl BridgeDiagnostics {
    pub fn new() -> Self {
        Self
    }

    pub async fn diagnose(&self) -> anyhow::Result<Vec<DiagnosticResult>> {
        let mut results = Vec::new();

        // Check for bridge-utils availability
        results.extend(self.check_bridge_tools().await?);

        // List all bridges
        results.extend(self.check_bridges().await?);

        // Check GhostWarden bridges specifically
        results.extend(self.check_ghostwarden_bridges().await?);

        // Check bridge forwarding and STP
        results.extend(self.check_bridge_configuration().await?);

        // Check for common misconfigurations
        results.extend(self.check_bridge_issues().await?);

        Ok(results)
    }

    async fn check_bridge_tools(&self) -> anyhow::Result<Vec<DiagnosticResult>> {
        let mut results = Vec::new();

        // Check if 'ip' command is available (iproute2)
        let ip_available = Command::new("ip")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if ip_available {
            results.push(DiagnosticResult::new(
                DiagnosticLevel::Info,
                "iproute2 tools available",
                "The 'ip' command is available for network configuration",
            ));
        } else {
            results.push(
                DiagnosticResult::new(
                    DiagnosticLevel::Critical,
                    "iproute2 not found",
                    "The 'ip' command is required but not found",
                )
                .with_suggestion("Install iproute2 package")
                .with_command("sudo pacman -S iproute2"),
            );
        }

        // Check if 'brctl' is available (optional, for compatibility)
        let brctl_available = Command::new("brctl")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if brctl_available {
            results.push(DiagnosticResult::new(
                DiagnosticLevel::Info,
                "bridge-utils available",
                "Legacy 'brctl' command is available",
            ));
        }

        Ok(results)
    }

    async fn check_bridges(&self) -> anyhow::Result<Vec<DiagnosticResult>> {
        let mut results = Vec::new();

        // List all bridge interfaces
        let output = Command::new("ip")
            .arg("link")
            .arg("show")
            .arg("type")
            .arg("bridge")
            .output()?;

        if !output.status.success() {
            results.push(
                DiagnosticResult::new(
                    DiagnosticLevel::Error,
                    "Failed to list bridges",
                    "Could not retrieve bridge interfaces",
                )
                .with_command("ip link show type bridge"),
            );
            return Ok(results);
        }

        let bridges_output = String::from_utf8_lossy(&output.stdout);

        // Parse bridge names
        let bridge_regex = Regex::new(r"^\d+:\s+([^:]+):").unwrap();
        let mut bridge_names = Vec::new();

        for line in bridges_output.lines() {
            if let Some(cap) = bridge_regex.captures(line) {
                bridge_names.push(cap[1].to_string());
            }
        }

        if bridge_names.is_empty() {
            results.push(
                DiagnosticResult::new(
                    DiagnosticLevel::Warning,
                    "No bridges found",
                    "No bridge interfaces are configured on this system",
                )
                .with_suggestion("Create bridges using 'gwarden net apply'"),
            );
        } else {
            results.push(DiagnosticResult::new(
                DiagnosticLevel::Info,
                "Bridges found",
                format!(
                    "Found {} bridge interface(s): {}",
                    bridge_names.len(),
                    bridge_names.join(", ")
                ),
            ));

            // Check each bridge in detail
            for bridge in &bridge_names {
                results.extend(self.inspect_bridge(bridge).await?);
            }
        }

        Ok(results)
    }

    async fn inspect_bridge(&self, bridge: &str) -> anyhow::Result<Vec<DiagnosticResult>> {
        let mut results = Vec::new();

        // Get bridge details
        let output = Command::new("ip")
            .arg("link")
            .arg("show")
            .arg(bridge)
            .output()?;

        if output.status.success() {
            let details = String::from_utf8_lossy(&output.stdout);

            // Check if bridge is UP
            if details.contains("state UP") {
                results.push(DiagnosticResult::new(
                    DiagnosticLevel::Info,
                    format!("Bridge {} status", bridge),
                    "State: UP",
                ));
            } else if details.contains("state DOWN") {
                results.push(
                    DiagnosticResult::new(
                        DiagnosticLevel::Warning,
                        format!("Bridge {} is DOWN", bridge),
                        "Bridge interface is not active",
                    )
                    .with_suggestion(format!("Bring up the bridge: ip link set {} up", bridge))
                    .with_command(format!("sudo ip link set {} up", bridge)),
                );
            }

            // Extract MTU
            if let Some(mtu) = details
                .split_whitespace()
                .skip_while(|s| *s != "mtu")
                .nth(1)
            {
                results.push(DiagnosticResult::new(
                    DiagnosticLevel::Info,
                    format!("Bridge {} MTU", bridge),
                    format!("MTU: {}", mtu),
                ));
            }
        }

        // Get IP addresses
        let output = Command::new("ip")
            .arg("addr")
            .arg("show")
            .arg(bridge)
            .output()?;

        if output.status.success() {
            let addr_output = String::from_utf8_lossy(&output.stdout);

            let mut has_ipv4 = false;
            for line in addr_output.lines() {
                if line.trim().starts_with("inet ") {
                    if let Some(ip) = line.trim().split_whitespace().nth(1) {
                        results.push(DiagnosticResult::new(
                            DiagnosticLevel::Info,
                            format!("Bridge {} IPv4", bridge),
                            format!("IP: {}", ip),
                        ));
                        has_ipv4 = true;
                    }
                }
            }

            if !has_ipv4 && bridge.starts_with("br-") {
                results.push(
                    DiagnosticResult::new(
                        DiagnosticLevel::Warning,
                        format!("Bridge {} has no IP address", bridge),
                        "Bridge may not function as a gateway",
                    )
                    .with_suggestion(
                        "Assign an IP address to the bridge if it should act as a gateway",
                    ),
                );
            }
        }

        // Get bridge ports/slaves
        let output = Command::new("ip")
            .arg("link")
            .arg("show")
            .arg("master")
            .arg(bridge)
            .output()?;

        if output.status.success() {
            let ports_output = String::from_utf8_lossy(&output.stdout);
            let port_regex = Regex::new(r"^\d+:\s+([^:@]+)").unwrap();
            let mut ports = Vec::new();

            for line in ports_output.lines() {
                if let Some(cap) = port_regex.captures(line) {
                    ports.push(cap[1].to_string());
                }
            }

            if ports.is_empty() {
                results.push(DiagnosticResult::new(
                    DiagnosticLevel::Info,
                    format!("Bridge {} ports", bridge),
                    "No ports attached",
                ));
            } else {
                results.push(DiagnosticResult::new(
                    DiagnosticLevel::Info,
                    format!("Bridge {} ports", bridge),
                    format!("{} port(s): {}", ports.len(), ports.join(", ")),
                ));
            }
        }

        Ok(results)
    }

    async fn check_ghostwarden_bridges(&self) -> anyhow::Result<Vec<DiagnosticResult>> {
        let mut results = Vec::new();

        // Look for br-* bridges (GhostWarden naming convention)
        let output = Command::new("ip")
            .arg("link")
            .arg("show")
            .arg("type")
            .arg("bridge")
            .output()?;

        if output.status.success() {
            let bridges = String::from_utf8_lossy(&output.stdout);
            let gw_bridges: Vec<&str> = bridges
                .lines()
                .filter_map(|line| {
                    if line.contains(": br-") {
                        line.split(':').nth(1)?.trim().split('@').next()
                    } else {
                        None
                    }
                })
                .collect();

            if gw_bridges.is_empty() {
                results.push(
                    DiagnosticResult::new(
                        DiagnosticLevel::Warning,
                        "No GhostWarden bridges found",
                        "No bridges matching 'br-*' pattern found",
                    )
                    .with_suggestion("Run 'gwarden net apply' to create network bridges"),
                );
            } else {
                results.push(DiagnosticResult::new(
                    DiagnosticLevel::Info,
                    "GhostWarden bridges detected",
                    format!(
                        "Found {} GhostWarden bridge(s): {}",
                        gw_bridges.len(),
                        gw_bridges.join(", ")
                    ),
                ));
            }
        }

        Ok(results)
    }

    async fn check_bridge_configuration(&self) -> anyhow::Result<Vec<DiagnosticResult>> {
        let mut results = Vec::new();

        // Check bridge netfilter settings
        let br_nf_settings = vec![
            (
                "net.bridge.bridge-nf-call-iptables",
                "Bridge iptables filtering",
            ),
            (
                "net.bridge.bridge-nf-call-ip6tables",
                "Bridge ip6tables filtering",
            ),
            (
                "net.bridge.bridge-nf-call-arptables",
                "Bridge arptables filtering",
            ),
        ];

        for (setting, description) in br_nf_settings {
            let output = Command::new("sysctl").arg(setting).output();

            if let Ok(output) = output {
                if output.status.success() {
                    let value = String::from_utf8_lossy(&output.stdout);

                    if value.contains("= 1") {
                        results.push(
                            DiagnosticResult::new(
                                DiagnosticLevel::Info,
                                format!("{} enabled", description),
                                value.trim().to_string(),
                            )
                            .with_suggestion(
                                "Bridge traffic will be filtered by iptables/nftables",
                            ),
                        );
                    } else {
                        results.push(DiagnosticResult::new(
                            DiagnosticLevel::Info,
                            format!("{} disabled", description),
                            value.trim().to_string(),
                        ));
                    }
                } else {
                    // br_netfilter module may not be loaded
                    results.push(
                        DiagnosticResult::new(
                            DiagnosticLevel::Warning,
                            format!("Cannot read {}", setting),
                            "br_netfilter module may not be loaded",
                        )
                        .with_suggestion("Load module: modprobe br_netfilter")
                        .with_command("sudo modprobe br_netfilter"),
                    );
                }
            }
        }

        Ok(results)
    }

    async fn check_bridge_issues(&self) -> anyhow::Result<Vec<DiagnosticResult>> {
        let mut results = Vec::new();

        // Check for common bridge problems

        // 1. Check if there are bridges with duplicate subnets
        let output = Command::new("ip")
            .arg("addr")
            .arg("show")
            .arg("type")
            .arg("bridge")
            .output()?;

        if output.status.success() {
            let addrs = String::from_utf8_lossy(&output.stdout);
            let ip_regex = Regex::new(r"inet\s+(\d+\.\d+\.\d+\.\d+/\d+)").unwrap();

            let mut seen_subnets: Vec<String> = Vec::new();
            let mut duplicates: Vec<String> = Vec::new();

            for cap in ip_regex.captures_iter(&addrs) {
                let subnet = &cap[1];

                // Extract network portion for comparison
                if let Some(network) = subnet.split('.').take(3).collect::<Vec<_>>().get(..3) {
                    let network_str = network.join(".");
                    if seen_subnets.contains(&network_str) {
                        duplicates.push(subnet.to_string());
                    } else {
                        seen_subnets.push(network_str);
                    }
                }
            }

            if !duplicates.is_empty() {
                results.push(
                    DiagnosticResult::new(
                        DiagnosticLevel::Warning,
                        "Potential subnet overlap detected",
                        format!("Found similar subnets: {}", duplicates.join(", ")),
                    )
                    .with_suggestion("Ensure bridge subnets don't overlap"),
                );
            }
        }

        // 2. Check for bridges without proper routing
        let output = Command::new("ip").arg("route").arg("show").output()?;

        if output.status.success() {
            let routes = String::from_utf8_lossy(&output.stdout);

            // Get list of bridges
            let bridge_output = Command::new("ip")
                .arg("link")
                .arg("show")
                .arg("type")
                .arg("bridge")
                .output()?;

            if bridge_output.status.success() {
                let bridges = String::from_utf8_lossy(&bridge_output.stdout);
                let bridge_regex = Regex::new(r"^\d+:\s+([^:]+):").unwrap();

                for cap in bridge_regex.captures_iter(&bridges) {
                    let bridge = &cap[1];

                    if !routes.contains(bridge) {
                        results.push(
                            DiagnosticResult::new(
                                DiagnosticLevel::Warning,
                                format!("No routes for bridge {}", bridge),
                                "Bridge has no associated routes",
                            )
                            .with_suggestion(
                                "Routes are typically auto-created when IP is assigned",
                            )
                            .with_command(format!("ip route show dev {}", bridge)),
                        );
                    }
                }
            }
        }

        // 3. Check for orphaned veth pairs
        let output = Command::new("ip")
            .arg("link")
            .arg("show")
            .arg("type")
            .arg("veth")
            .output()?;

        if output.status.success() {
            let veths = String::from_utf8_lossy(&output.stdout);
            let veth_regex = Regex::new(r"^\d+:\s+([^:@]+)").unwrap();
            let veth_count = veth_regex.captures_iter(&veths).count();

            if veth_count > 0 {
                results.push(
                    DiagnosticResult::new(
                        DiagnosticLevel::Info,
                        "veth pairs detected",
                        format!("Found {} veth interface(s)", veth_count / 2),
                    )
                    .with_suggestion("veth pairs are used for container/VM networking"),
                );
            }

            // Check for DOWN veth interfaces (potential orphans)
            let down_veths: Vec<&str> = veths
                .lines()
                .filter(|line| line.contains("state DOWN"))
                .filter_map(|line| {
                    veth_regex
                        .captures(line)
                        .and_then(|cap| cap.get(1))
                        .map(|m| m.as_str())
                })
                .collect();

            if !down_veths.is_empty() {
                results.push(
                    DiagnosticResult::new(
                        DiagnosticLevel::Warning,
                        "Inactive veth interfaces found",
                        format!(
                            "Found {} DOWN veth interface(s): {}",
                            down_veths.len(),
                            down_veths.join(", ")
                        ),
                    )
                    .with_suggestion("These may be orphaned interfaces from stopped containers/VMs")
                    .with_command("ip link show type veth"),
                );
            }
        }

        Ok(results)
    }
}

impl Default for BridgeDiagnostics {
    fn default() -> Self {
        Self::new()
    }
}
