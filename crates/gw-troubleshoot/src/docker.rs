use crate::diagnostics::{DiagnosticLevel, DiagnosticResult};
use serde::Deserialize;
use std::process::Command;

/// Docker networking diagnostics
pub struct DockerDiagnostics {
    docker_available: bool,
}

#[derive(Debug, Deserialize)]
struct DockerNetwork {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Driver")]
    driver: String,
    #[serde(rename = "Scope")]
    scope: String,
}

impl DockerDiagnostics {
    pub fn new() -> Self {
        let docker_available = Command::new("docker")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        Self { docker_available }
    }

    pub async fn diagnose(&self) -> anyhow::Result<Vec<DiagnosticResult>> {
        let mut results = Vec::new();

        if !self.docker_available {
            results.push(DiagnosticResult::new(
                DiagnosticLevel::Info,
                "Docker not found",
                "Docker is not installed or not in PATH",
            ));
            return Ok(results);
        }

        // Check Docker daemon status
        results.extend(self.check_docker_daemon().await?);

        // Check Docker networks
        results.extend(self.check_docker_networks().await?);

        // Check Docker bridge configuration
        results.extend(self.check_docker_bridge().await?);

        // Check for network conflicts
        results.extend(self.check_network_conflicts().await?);

        // Check Docker iptables integration
        results.extend(self.check_docker_iptables().await?);

        Ok(results)
    }

    async fn check_docker_daemon(&self) -> anyhow::Result<Vec<DiagnosticResult>> {
        let mut results = Vec::new();

        let output = Command::new("docker")
            .arg("info")
            .arg("--format")
            .arg("{{json .}}")
            .output()?;

        if !output.status.success() {
            results.push(
                DiagnosticResult::new(
                    DiagnosticLevel::Warning,
                    "Docker daemon not accessible",
                    "Cannot connect to Docker daemon",
                )
                .with_suggestion("Ensure Docker daemon is running")
                .with_command("sudo systemctl status docker"),
            );
            return Ok(results);
        }

        results.push(DiagnosticResult::new(
            DiagnosticLevel::Info,
            "Docker daemon running",
            "Docker daemon is accessible",
        ));

        // Parse Docker info
        let info = String::from_utf8_lossy(&output.stdout);

        // Check for iptables mode
        if info.contains("\"iptables\": true") || info.contains("\"Iptables\": true") {
            results.push(
                DiagnosticResult::new(
                    DiagnosticLevel::Info,
                    "Docker iptables integration enabled",
                    "Docker is managing iptables rules",
                )
                .with_suggestion("This may interact with nftables - ensure proper rule precedence"),
            );
        }

        Ok(results)
    }

    async fn check_docker_networks(&self) -> anyhow::Result<Vec<DiagnosticResult>> {
        let mut results = Vec::new();

        let output = Command::new("docker")
            .arg("network")
            .arg("ls")
            .arg("--format")
            .arg("{{json .}}")
            .output()?;

        if !output.status.success() {
            return Ok(results);
        }

        let networks_output = String::from_utf8_lossy(&output.stdout);
        let network_count = networks_output.lines().count();

        results.push(DiagnosticResult::new(
            DiagnosticLevel::Info,
            "Docker networks found",
            format!("Found {} Docker network(s)", network_count),
        ));

        // Parse networks
        for line in networks_output.lines() {
            if let Ok(network) = serde_json::from_str::<DockerNetwork>(line) {
                if network.driver == "bridge" && network.name != "bridge" {
                    results.push(DiagnosticResult::new(
                        DiagnosticLevel::Info,
                        format!("Custom bridge network: {}", network.name),
                        format!("Driver: {}, Scope: {}", network.driver, network.scope),
                    ));
                }
            }
        }

        Ok(results)
    }

    async fn check_docker_bridge(&self) -> anyhow::Result<Vec<DiagnosticResult>> {
        let mut results = Vec::new();

        // Check if docker0 bridge exists
        let output = Command::new("ip")
            .arg("link")
            .arg("show")
            .arg("docker0")
            .output()?;

        if output.status.success() {
            results.push(DiagnosticResult::new(
                DiagnosticLevel::Info,
                "Docker bridge (docker0) exists",
                "Default Docker bridge interface is present",
            ));

            // Get docker0 IP address
            let output = Command::new("ip")
                .arg("addr")
                .arg("show")
                .arg("docker0")
                .output()?;

            if output.status.success() {
                let addr_info = String::from_utf8_lossy(&output.stdout);

                // Extract IP address
                for line in addr_info.lines() {
                    if line.contains("inet ") {
                        let ip = line.trim().split_whitespace().nth(1).unwrap_or("unknown");

                        results.push(DiagnosticResult::new(
                            DiagnosticLevel::Info,
                            "Docker bridge IP",
                            format!("docker0: {}", ip),
                        ));
                    }
                }

                // Check if interface is up
                if addr_info.contains("state UP") {
                    results.push(DiagnosticResult::new(
                        DiagnosticLevel::Info,
                        "Docker bridge status",
                        "docker0 is UP",
                    ));
                } else {
                    results.push(
                        DiagnosticResult::new(
                            DiagnosticLevel::Warning,
                            "Docker bridge down",
                            "docker0 is not in UP state",
                        )
                        .with_suggestion("Docker bridge may not be functioning properly"),
                    );
                }
            }
        } else {
            results.push(
                DiagnosticResult::new(
                    DiagnosticLevel::Warning,
                    "Docker bridge (docker0) not found",
                    "Default Docker bridge is missing",
                )
                .with_suggestion("Docker may not be properly configured"),
            );
        }

        Ok(results)
    }

    async fn check_network_conflicts(&self) -> anyhow::Result<Vec<DiagnosticResult>> {
        let mut results = Vec::new();

        // Get Docker network subnets
        let output = Command::new("docker")
            .arg("network")
            .arg("inspect")
            .arg("bridge")
            .output()?;

        if output.status.success() {
            let info = String::from_utf8_lossy(&output.stdout);

            // Parse subnet information
            if let Ok(networks) = serde_json::from_str::<serde_json::Value>(&info) {
                if let Some(ipam) = networks[0].get("IPAM") {
                    if let Some(config) = ipam.get("Config") {
                        if let Some(subnet) = config[0].get("Subnet") {
                            let subnet_str = subnet.as_str().unwrap_or("unknown");

                            results.push(DiagnosticResult::new(
                                DiagnosticLevel::Info,
                                "Docker bridge subnet",
                                format!("Default bridge subnet: {}", subnet_str),
                            ));

                            // Check for common conflicts (e.g., 172.17.0.0/16)
                            if subnet_str.starts_with("172.17.") {
                                results.push(
                                    DiagnosticResult::new(
                                        DiagnosticLevel::Warning,
                                        "Common Docker subnet detected",
                                        "Using default 172.17.0.0/16 - may conflict with VPNs or corporate networks"
                                    )
                                    .with_suggestion("Consider configuring a custom subnet in /etc/docker/daemon.json")
                                );
                            }

                            // Check if it overlaps with common private ranges used by GhostWarden
                            if subnet_str.starts_with("10.") || subnet_str.starts_with("192.168.") {
                                results.push(
                                    DiagnosticResult::new(
                                        DiagnosticLevel::Warning,
                                        "Potential subnet conflict",
                                        format!("Docker subnet {} may overlap with GhostWarden networks", subnet_str)
                                    )
                                    .with_suggestion("Ensure Docker and GhostWarden use non-overlapping subnets")
                                );
                            }
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    async fn check_docker_iptables(&self) -> anyhow::Result<Vec<DiagnosticResult>> {
        let mut results = Vec::new();

        // Check if iptables has DOCKER chains
        let output = Command::new("iptables").arg("-L").arg("-n").output();

        if let Ok(output) = output {
            if output.status.success() {
                let rules = String::from_utf8_lossy(&output.stdout);

                if rules.contains("Chain DOCKER") {
                    results.push(DiagnosticResult::new(
                        DiagnosticLevel::Info,
                        "Docker iptables chains found",
                        "Docker is managing its own iptables chains",
                    ));

                    // Check DOCKER-USER chain
                    if rules.contains("Chain DOCKER-USER") {
                        results.push(
                            DiagnosticResult::new(
                                DiagnosticLevel::Info,
                                "DOCKER-USER chain available",
                                "You can add custom rules to DOCKER-USER chain"
                            )
                            .with_suggestion("Use DOCKER-USER for custom firewall rules that should apply to Docker containers")
                        );
                    }
                } else {
                    results.push(
                        DiagnosticResult::new(
                            DiagnosticLevel::Warning,
                            "No Docker iptables chains",
                            "Docker chains not found - iptables integration may be disabled",
                        )
                        .with_suggestion("Check Docker daemon configuration for iptables setting"),
                    );
                }

                // Check for DOCKER-ISOLATION chains
                if rules.contains("DOCKER-ISOLATION") {
                    results.push(DiagnosticResult::new(
                        DiagnosticLevel::Info,
                        "Docker network isolation active",
                        "DOCKER-ISOLATION chain is managing network separation",
                    ));
                }
            }
        }

        // Check NAT rules
        let output = Command::new("iptables")
            .arg("-t")
            .arg("nat")
            .arg("-L")
            .arg("DOCKER")
            .arg("-n")
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                let rules = String::from_utf8_lossy(&output.stdout);
                let rule_count = rules
                    .lines()
                    .filter(|line| !line.starts_with("Chain") && !line.starts_with("target"))
                    .filter(|line| !line.trim().is_empty())
                    .count();

                if rule_count > 0 {
                    results.push(
                        DiagnosticResult::new(
                            DiagnosticLevel::Info,
                            "Docker NAT rules active",
                            format!("Found {} Docker NAT rule(s)", rule_count),
                        )
                        .with_command("iptables -t nat -L DOCKER -n -v"),
                    );
                }
            }
        }

        Ok(results)
    }
}

impl Default for DockerDiagnostics {
    fn default() -> Self {
        Self::new()
    }
}
