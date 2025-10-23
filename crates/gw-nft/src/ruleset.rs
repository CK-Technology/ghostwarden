use anyhow::{Context, Result};
use serde_json::json;
use gw_core::policy::{PolicyProfile, Protocol, Action};

pub struct NftManager;

impl NftManager {
    pub fn new() -> Self {
        Self
    }

    /// Generate a complete nftables ruleset for NAT/routing
    pub fn create_nat_ruleset(
        &self,
        table_name: &str,
        bridge_cidr: &str,
        masq_iface: &str,
        forwards: &[(String, String)], // (public, dst) pairs
    ) -> Result<String> {
        let mut nftables = vec![
            // Flush existing table if it exists
            json!({"flush": {"table": {"family": "inet", "name": table_name}}}),
            // Create table
            json!({"table": {"family": "inet", "name": table_name}}),
        ];

        // Add base chains
        nftables.push(json!({
            "chain": {
                "family": "inet",
                "table": table_name,
                "name": "forward",
                "type": "filter",
                "hook": "forward",
                "prio": 0,
                "policy": "accept"
            }
        }));

        nftables.push(json!({
            "chain": {
                "family": "inet",
                "table": table_name,
                "name": "postrouting",
                "type": "nat",
                "hook": "postrouting",
                "prio": 100,
                "policy": "accept"
            }
        }));

        nftables.push(json!({
            "chain": {
                "family": "inet",
                "table": table_name,
                "name": "prerouting",
                "type": "nat",
                "hook": "prerouting",
                "prio": -100,
                "policy": "accept"
            }
        }));

        // Add MASQUERADE rule for outbound NAT
        nftables.push(json!({
            "rule": {
                "family": "inet",
                "table": table_name,
                "chain": "postrouting",
                "expr": [
                    {"match": {"left": {"meta": {"key": "oifname"}}, "op": "==", "right": masq_iface}},
                    {"match": {"left": {"payload": {"protocol": "ip", "field": "saddr"}}, "op": "==", "right": {"prefix": {"addr": bridge_cidr.split('/').next().unwrap(), "len": bridge_cidr.split('/').nth(1).unwrap().parse::<u32>().unwrap()}}}},
                    {"masquerade": null}
                ]
            }
        }));

        // Add DNAT rules for port forwards
        for (public, dst) in forwards {
            let (_pub_addr, pub_port_proto) = parse_public_addr(public)?;
            let (pub_port, proto) = parse_port_proto(&pub_port_proto)?;
            let (dst_addr, dst_port) = parse_dest(dst)?;

            nftables.push(json!({
                "rule": {
                    "family": "inet",
                    "table": table_name,
                    "chain": "prerouting",
                    "expr": [
                        {"match": {"left": {"payload": {"protocol": proto, "field": "dport"}}, "op": "==", "right": pub_port}},
                        {"dnat": {"addr": dst_addr, "port": dst_port}}
                    ]
                }
            }));
        }

        let ruleset = json!({"nftables": nftables});
        Ok(serde_json::to_string_pretty(&ruleset)?)
    }

    /// Generate nftables filter rules from a policy profile
    pub fn create_policy_ruleset(
        &self,
        table_name: &str,
        bridge_name: &str,
        policy: &PolicyProfile,
    ) -> Result<String> {
        let mut nftables = vec![
            // Flush existing table if it exists
            json!({"flush": {"table": {"family": "inet", "name": table_name}}}),
            // Create table
            json!({"table": {"family": "inet", "name": table_name}}),
        ];

        // Convert default_action to nftables policy
        let default_policy = match policy.default_action {
            Action::Accept => "accept",
            Action::Drop => "drop",
            Action::Reject => "drop", // nftables base chain policy doesn't support reject, use drop
        };

        // Add input chain with default policy
        nftables.push(json!({
            "chain": {
                "family": "inet",
                "table": table_name,
                "name": "input",
                "type": "filter",
                "hook": "input",
                "prio": 0,
                "policy": default_policy
            }
        }));

        // Add forward chain
        nftables.push(json!({
            "chain": {
                "family": "inet",
                "table": table_name,
                "name": "forward",
                "type": "filter",
                "hook": "forward",
                "prio": 0,
                "policy": default_policy
            }
        }));

        // Add output chain (usually permissive)
        nftables.push(json!({
            "chain": {
                "family": "inet",
                "table": table_name,
                "name": "output",
                "type": "filter",
                "hook": "output",
                "prio": 0,
                "policy": "accept"
            }
        }));

        // Add postrouting chain for NAT
        nftables.push(json!({
            "chain": {
                "family": "inet",
                "table": table_name,
                "name": "postrouting",
                "type": "nat",
                "hook": "postrouting",
                "prio": 100,
                "policy": "accept"
            }
        }));

        // Add prerouting chain for NAT
        nftables.push(json!({
            "chain": {
                "family": "inet",
                "table": table_name,
                "name": "prerouting",
                "type": "nat",
                "hook": "prerouting",
                "prio": -100,
                "policy": "accept"
            }
        }));

        // Allow established/related connections (critical for stateful firewall)
        nftables.push(json!({
            "rule": {
                "family": "inet",
                "table": table_name,
                "chain": "input",
                "expr": [
                    {"match": {"left": {"ct": {"key": "state"}}, "op": "in", "right": ["established", "related"]}},
                    {"accept": null}
                ]
            }
        }));

        nftables.push(json!({
            "rule": {
                "family": "inet",
                "table": table_name,
                "chain": "forward",
                "expr": [
                    {"match": {"left": {"ct": {"key": "state"}}, "op": "in", "right": ["established", "related"]}},
                    {"accept": null}
                ]
            }
        }));

        // Allow loopback
        nftables.push(json!({
            "rule": {
                "family": "inet",
                "table": table_name,
                "chain": "input",
                "expr": [
                    {"match": {"left": {"meta": {"key": "iifname"}}, "op": "==", "right": "lo"}},
                    {"accept": null}
                ]
            }
        }));

        // Add service rules (ingress)
        for service in &policy.services {
            let proto = match service.protocol {
                Protocol::Tcp => "tcp",
                Protocol::Udp => "udp",
                Protocol::Icmp => "icmp",
            };

            // Build expression based on protocol
            let mut expr = vec![];

            // Add interface match for bridge
            expr.push(json!({"match": {"left": {"meta": {"key": "iifname"}}, "op": "==", "right": bridge_name}}));

            // Add protocol match (except for ICMP)
            if !matches!(service.protocol, Protocol::Icmp) {
                expr.push(json!({"match": {"left": {"meta": {"key": "l4proto"}}, "op": "==", "right": proto}}));
                expr.push(json!({"match": {"left": {"payload": {"protocol": proto, "field": "dport"}}, "op": "==", "right": service.port}}));
            } else {
                expr.push(json!({"match": {"left": {"meta": {"key": "l4proto"}}, "op": "==", "right": "icmp"}}));
            }

            // Add source CIDR match if specified
            if let Some(ref source) = service.source {
                expr.push(json!({"match": {"left": {"payload": {"protocol": "ip", "field": "saddr"}}, "op": "==", "right": source}}));
            }

            expr.push(json!({"accept": null}));

            nftables.push(json!({
                "rule": {
                    "family": "inet",
                    "table": table_name,
                    "chain": "input",
                    "expr": expr
                }
            }));
        }

        // Add ingress CIDR allow rules
        for cidr in &policy.allowed_ingress_cidrs {
            nftables.push(json!({
                "rule": {
                    "family": "inet",
                    "table": table_name,
                    "chain": "input",
                    "expr": [
                        {"match": {"left": {"meta": {"key": "iifname"}}, "op": "==", "right": bridge_name}},
                        {"match": {"left": {"payload": {"protocol": "ip", "field": "saddr"}}, "op": "==", "right": cidr}},
                        {"accept": null}
                    ]
                }
            }));
        }

        // Add egress CIDR allow rules (for forward chain)
        for cidr in &policy.allowed_egress_cidrs {
            nftables.push(json!({
                "rule": {
                    "family": "inet",
                    "table": table_name,
                    "chain": "forward",
                    "expr": [
                        {"match": {"left": {"meta": {"key": "iifname"}}, "op": "==", "right": bridge_name}},
                        {"match": {"left": {"payload": {"protocol": "ip", "field": "daddr"}}, "op": "==", "right": cidr}},
                        {"accept": null}
                    ]
                }
            }));
        }

        let ruleset = json!({"nftables": nftables});
        Ok(serde_json::to_string_pretty(&ruleset)?)
    }

    /// Generate a complete ruleset with NAT + policy filtering
    pub fn create_complete_ruleset(
        &self,
        table_name: &str,
        bridge_name: &str,
        bridge_cidr: &str,
        masq_iface: &str,
        forwards: &[(String, String)],
        policy: Option<&PolicyProfile>,
    ) -> Result<String> {
        let mut nftables = vec![
            // Flush existing table if it exists
            json!({"flush": {"table": {"family": "inet", "name": table_name}}}),
            // Create table
            json!({"table": {"family": "inet", "name": table_name}}),
        ];

        // Determine default policy
        let default_policy = if let Some(p) = policy {
            match p.default_action {
                Action::Accept => "accept",
                Action::Drop => "drop",
                Action::Reject => "drop",
            }
        } else {
            "accept"
        };

        // Add chains
        nftables.push(json!({
            "chain": {
                "family": "inet",
                "table": table_name,
                "name": "input",
                "type": "filter",
                "hook": "input",
                "prio": 0,
                "policy": default_policy
            }
        }));

        nftables.push(json!({
            "chain": {
                "family": "inet",
                "table": table_name,
                "name": "forward",
                "type": "filter",
                "hook": "forward",
                "prio": 0,
                "policy": default_policy
            }
        }));

        nftables.push(json!({
            "chain": {
                "family": "inet",
                "table": table_name,
                "name": "output",
                "type": "filter",
                "hook": "output",
                "prio": 0,
                "policy": "accept"
            }
        }));

        nftables.push(json!({
            "chain": {
                "family": "inet",
                "table": table_name,
                "name": "postrouting",
                "type": "nat",
                "hook": "postrouting",
                "prio": 100,
                "policy": "accept"
            }
        }));

        nftables.push(json!({
            "chain": {
                "family": "inet",
                "table": table_name,
                "name": "prerouting",
                "type": "nat",
                "hook": "prerouting",
                "prio": -100,
                "policy": "accept"
            }
        }));

        // Stateful firewall rules (established/related)
        nftables.push(json!({
            "rule": {
                "family": "inet",
                "table": table_name,
                "chain": "input",
                "expr": [
                    {"match": {"left": {"ct": {"key": "state"}}, "op": "in", "right": ["established", "related"]}},
                    {"accept": null}
                ]
            }
        }));

        nftables.push(json!({
            "rule": {
                "family": "inet",
                "table": table_name,
                "chain": "forward",
                "expr": [
                    {"match": {"left": {"ct": {"key": "state"}}, "op": "in", "right": ["established", "related"]}},
                    {"accept": null}
                ]
            }
        }));

        // Allow loopback
        nftables.push(json!({
            "rule": {
                "family": "inet",
                "table": table_name,
                "chain": "input",
                "expr": [
                    {"match": {"left": {"meta": {"key": "iifname"}}, "op": "==", "right": "lo"}},
                    {"accept": null}
                ]
            }
        }));

        // Add policy-based rules if policy is provided
        if let Some(policy) = policy {
            // Service rules
            for service in &policy.services {
                let proto = match service.protocol {
                    Protocol::Tcp => "tcp",
                    Protocol::Udp => "udp",
                    Protocol::Icmp => "icmp",
                };

                let mut expr = vec![];
                expr.push(json!({"match": {"left": {"meta": {"key": "iifname"}}, "op": "==", "right": bridge_name}}));

                if !matches!(service.protocol, Protocol::Icmp) {
                    expr.push(json!({"match": {"left": {"meta": {"key": "l4proto"}}, "op": "==", "right": proto}}));
                    expr.push(json!({"match": {"left": {"payload": {"protocol": proto, "field": "dport"}}, "op": "==", "right": service.port}}));
                } else {
                    expr.push(json!({"match": {"left": {"meta": {"key": "l4proto"}}, "op": "==", "right": "icmp"}}));
                }

                if let Some(ref source) = service.source {
                    expr.push(json!({"match": {"left": {"payload": {"protocol": "ip", "field": "saddr"}}, "op": "==", "right": source}}));
                }

                expr.push(json!({"accept": null}));

                nftables.push(json!({
                    "rule": {
                        "family": "inet",
                        "table": table_name,
                        "chain": "input",
                        "expr": expr
                    }
                }));
            }

            // Ingress CIDR rules
            for cidr in &policy.allowed_ingress_cidrs {
                nftables.push(json!({
                    "rule": {
                        "family": "inet",
                        "table": table_name,
                        "chain": "input",
                        "expr": [
                            {"match": {"left": {"meta": {"key": "iifname"}}, "op": "==", "right": bridge_name}},
                            {"match": {"left": {"payload": {"protocol": "ip", "field": "saddr"}}, "op": "==", "right": cidr}},
                            {"accept": null}
                        ]
                    }
                }));
            }

            // Egress CIDR rules (forward chain)
            for cidr in &policy.allowed_egress_cidrs {
                nftables.push(json!({
                    "rule": {
                        "family": "inet",
                        "table": table_name,
                        "chain": "forward",
                        "expr": [
                            {"match": {"left": {"meta": {"key": "iifname"}}, "op": "==", "right": bridge_name}},
                            {"match": {"left": {"payload": {"protocol": "ip", "field": "daddr"}}, "op": "==", "right": cidr}},
                            {"accept": null}
                        ]
                    }
                }));
            }
        }

        // Add MASQUERADE rule for NAT
        if !masq_iface.is_empty() {
            nftables.push(json!({
                "rule": {
                    "family": "inet",
                    "table": table_name,
                    "chain": "postrouting",
                    "expr": [
                        {"match": {"left": {"meta": {"key": "oifname"}}, "op": "==", "right": masq_iface}},
                        {"match": {"left": {"payload": {"protocol": "ip", "field": "saddr"}}, "op": "==", "right": {"prefix": {"addr": bridge_cidr.split('/').next().unwrap(), "len": bridge_cidr.split('/').nth(1).unwrap().parse::<u32>().unwrap()}}}},
                        {"masquerade": null}
                    ]
                }
            }));
        }

        // Add DNAT rules for port forwards
        for (public, dst) in forwards {
            let (_pub_addr, pub_port_proto) = parse_public_addr(public)?;
            let (pub_port, proto) = parse_port_proto(&pub_port_proto)?;
            let (dst_addr, dst_port) = parse_dest(dst)?;

            nftables.push(json!({
                "rule": {
                    "family": "inet",
                    "table": table_name,
                    "chain": "prerouting",
                    "expr": [
                        {"match": {"left": {"payload": {"protocol": &proto, "field": "dport"}}, "op": "==", "right": pub_port}},
                        {"dnat": {"addr": dst_addr, "port": dst_port}}
                    ]
                }
            }));
        }

        let ruleset = json!({"nftables": nftables});
        Ok(serde_json::to_string_pretty(&ruleset)?)
    }

    /// Apply nftables ruleset using `nft -j -f`
    pub async fn apply_ruleset(&self, ruleset: &str) -> Result<()> {
        use tokio::process::Command;
        use tokio::io::AsyncWriteExt;

        let mut child = Command::new("nft")
            .arg("-j")
            .arg("-f")
            .arg("-")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("Failed to spawn nft command")?;

        // Write ruleset to stdin
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(ruleset.as_bytes()).await?;
            drop(stdin);
        }

        let output = child.wait_with_output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("nft command failed: {}", stderr);
        }

        println!("Applied nftables ruleset");
        Ok(())
    }

    /// Delete nftables table
    pub async fn delete_table(&self, table_name: &str) -> Result<()> {
        use tokio::process::Command;

        let output = Command::new("nft")
            .arg("delete")
            .arg("table")
            .arg("inet")
            .arg(table_name)
            .output()
            .await
            .context("Failed to run nft delete table")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Ignore "No such file or directory" errors (table doesn't exist)
            if !stderr.contains("No such file or directory") {
                anyhow::bail!("Failed to delete table: {}", stderr);
            }
        }

        println!("Deleted nftables table: {}", table_name);
        Ok(())
    }

    /// List existing tables
    pub async fn list_tables(&self) -> Result<Vec<String>> {
        use tokio::process::Command;

        let output = Command::new("nft")
            .arg("-j")
            .arg("list")
            .arg("tables")
            .output()
            .await
            .context("Failed to list nftables tables")?;

        if !output.status.success() {
            return Ok(vec![]);
        }

        let json_output = String::from_utf8_lossy(&output.stdout);
        let parsed: serde_json::Value = serde_json::from_str(&json_output)?;

        let mut tables = vec![];
        if let Some(nftables) = parsed.get("nftables").and_then(|n| n.as_array()) {
            for item in nftables {
                if let Some(table) = item.get("table") {
                    if let Some(name) = table.get("name").and_then(|n| n.as_str()) {
                        tables.push(name.to_string());
                    }
                }
            }
        }

        Ok(tables)
    }
}

impl Default for NftManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse "0.0.0.0:4022/tcp" -> ("0.0.0.0", "4022/tcp")
fn parse_public_addr(public: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = public.splitn(2, ':').collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid public address format: {}", public);
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

/// Parse "4022/tcp" -> (4022, "tcp")
fn parse_port_proto(port_proto: &str) -> Result<(u16, String)> {
    let parts: Vec<&str> = port_proto.split('/').collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid port/proto format: {}", port_proto);
    }
    let port: u16 = parts[0].parse().context("Invalid port number")?;
    Ok((port, parts[1].to_string()))
}

/// Parse "10.33.0.10:22" -> ("10.33.0.10", 22)
fn parse_dest(dst: &str) -> Result<(String, u16)> {
    let parts: Vec<&str> = dst.splitn(2, ':').collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid destination format: {}", dst);
    }
    let port: u16 = parts[1].parse().context("Invalid destination port")?;
    Ok((parts[0].to_string(), port))
}
