use anyhow::{Context, Result, anyhow};
use gw_core::policy::{Action, PolicyProfile, Protocol};
use ipnet::IpNet;
use serde_json::{Value, json};
use similar::{ChangeTag, TextDiff};
use std::net::IpAddr;

pub struct NftManager;

pub struct NftDiff {
    pub table: String,
    pub matches: bool,
    pub current_exists: bool,
    pub diff: Option<String>,
}

impl NftManager {
    pub fn new() -> Self {
        Self
    }

    /// Generate a complete nftables ruleset for NAT/routing
    pub fn create_nat_ruleset(
        &self,
        table_name: &str,
        bridge_name: &str,
        bridge_cidr: &str,
        gateway_ip: &str,
        masq_iface: &str,
        forwards: &[(String, String)], // (public, dst) pairs
    ) -> Result<String> {
        let bridge_net: IpNet = bridge_cidr
            .parse()
            .with_context(|| format!("Invalid bridge CIDR '{}'", bridge_cidr))?;
        let gateway: IpAddr = gateway_ip
            .parse()
            .with_context(|| format!("Invalid gateway IP '{}'", gateway_ip))?;
        let parsed_forwards = parse_forward_rules(forwards)?;

        let mut nftables = base_table_definition(table_name);

        nftables.extend(base_filter_chain(table_name, "accept"));
        nftables.extend(base_nat_chains(table_name));
        nftables.extend(build_nat_rules(
            table_name,
            bridge_name,
            &bridge_net,
            &gateway,
            masq_iface,
            &parsed_forwards,
        ));

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
        gateway_ip: &str,
        masq_iface: &str,
        forwards: &[(String, String)],
        policy: Option<&PolicyProfile>,
    ) -> Result<String> {
        let bridge_net: IpNet = bridge_cidr
            .parse()
            .with_context(|| format!("Invalid bridge CIDR '{}'", bridge_cidr))?;
        let gateway: IpAddr = gateway_ip
            .parse()
            .with_context(|| format!("Invalid gateway IP '{}'", gateway_ip))?;
        let parsed_forwards = parse_forward_rules(forwards)?;

        let mut nftables = base_table_definition(table_name);

        let default_policy = policy
            .map(|p| match p.default_action {
                Action::Accept => "accept",
                Action::Drop | Action::Reject => "drop",
            })
            .unwrap_or("accept");

        nftables.extend(base_filter_chain(table_name, default_policy));
        nftables.extend(base_output_chain(table_name));
        nftables.extend(base_nat_chains(table_name));

        nftables.extend(stateful_allow_rules(table_name));
        nftables.push(loopback_rule(table_name));

        if let Some(policy) = policy {
            nftables.extend(policy_service_rules(table_name, bridge_name, policy)?);
            nftables.extend(policy_ingress_rules(table_name, bridge_name, policy)?);
            nftables.extend(policy_egress_rules(table_name, bridge_name, policy)?);
        }

        nftables.extend(build_nat_rules(
            table_name,
            bridge_name,
            &bridge_net,
            &gateway,
            masq_iface,
            &parsed_forwards,
        ));

        let ruleset = json!({"nftables": nftables});
        Ok(serde_json::to_string_pretty(&ruleset)?)
    }

    /// Compare desired ruleset with the live table and return a textual diff
    pub async fn diff_ruleset(&self, table_name: &str, desired_ruleset: &str) -> Result<NftDiff> {
        let normalized_desired = normalize_json(desired_ruleset)
            .with_context(|| format!("Failed to normalize desired ruleset for {}", table_name))?;

        match self.snapshot_table(table_name).await? {
            Some(current_raw) => {
                let normalized_current = normalize_json(&current_raw)
                    .with_context(|| format!("Failed to parse live ruleset for {}", table_name))?;

                if normalized_current == normalized_desired {
                    Ok(NftDiff {
                        table: table_name.to_string(),
                        matches: true,
                        current_exists: true,
                        diff: None,
                    })
                } else {
                    let diff_text = render_diff(&normalized_current, &normalized_desired);
                    Ok(NftDiff {
                        table: table_name.to_string(),
                        matches: false,
                        current_exists: true,
                        diff: Some(diff_text),
                    })
                }
            }
            None => {
                let diff_text = render_diff("", &normalized_desired);
                Ok(NftDiff {
                    table: table_name.to_string(),
                    matches: false,
                    current_exists: false,
                    diff: Some(diff_text),
                })
            }
        }
    }

    /// Apply nftables ruleset using `nft -j -f`, returning the previous snapshot (if any)
    pub async fn apply_ruleset(&self, table_name: &str, ruleset: &str) -> Result<Option<String>> {
        let snapshot = self.snapshot_table(table_name).await?;
        self.apply_ruleset_payload(ruleset).await?;
        println!("Applied nftables ruleset");
        Ok(snapshot)
    }

    async fn apply_ruleset_payload(&self, payload: &str) -> Result<()> {
        use tokio::io::AsyncWriteExt;
        use tokio::process::Command;

        let mut child = Command::new("nft")
            .arg("-j")
            .arg("-f")
            .arg("-")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("Failed to spawn nft command")?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(payload.as_bytes()).await?;
            drop(stdin);
        }

        let output = child.wait_with_output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("nft command failed: {}", stderr);
        }

        Ok(())
    }

    /// Snapshot an existing table, returning the JSON definition if it exists
    pub async fn snapshot_table(&self, table_name: &str) -> Result<Option<String>> {
        use tokio::process::Command;

        let output = Command::new("nft")
            .arg("-j")
            .arg("list")
            .arg("table")
            .arg("inet")
            .arg(table_name)
            .output()
            .await
            .context("Failed to list nftables table")?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            return Ok(Some(stdout));
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("No such file or directory") {
            return Ok(None);
        }

        anyhow::bail!("Failed to snapshot table {}: {}", table_name, stderr);
    }

    /// Restore a table from a snapshot, or delete it if no snapshot existed
    pub async fn restore_table_from_snapshot(
        &self,
        table_name: &str,
        snapshot: Option<&str>,
    ) -> Result<()> {
        match snapshot {
            Some(data) => {
                self.apply_ruleset_payload(data).await?;
                println!("Restored nftables table: {}", table_name);
            }
            None => {
                self.delete_table(table_name).await?;
            }
        }

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

fn normalize_json(input: &str) -> Result<String> {
    let value: serde_json::Value =
        serde_json::from_str(input).with_context(|| "Failed to parse nftables JSON payload")?;
    Ok(serde_json::to_string_pretty(&value)?)
}

fn render_diff(current: &str, desired: &str) -> String {
    let diff = TextDiff::from_lines(current, desired);
    let mut output = String::new();

    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Delete => '-',
            ChangeTag::Insert => '+',
            ChangeTag::Equal => ' ',
        };
        output.push(sign);
        output.push_str(change.value());
    }

    output
}

fn base_table_definition(table_name: &str) -> Vec<Value> {
    vec![
        json!({"flush": {"table": {"family": "inet", "name": table_name}}}),
        json!({"table": {"family": "inet", "name": table_name}}),
    ]
}

fn base_filter_chain(table_name: &str, default_policy: &str) -> Vec<Value> {
    vec![
        json!({
            "chain": {
                "family": "inet",
                "table": table_name,
                "name": "input",
                "type": "filter",
                "hook": "input",
                "prio": 0,
                "policy": default_policy,
            },
        }),
        json!({
            "chain": {
                "family": "inet",
                "table": table_name,
                "name": "forward",
                "type": "filter",
                "hook": "forward",
                "prio": 0,
                "policy": default_policy,
            },
        }),
    ]
}

fn base_output_chain(table_name: &str) -> Vec<Value> {
    vec![json!({
        "chain": {
            "family": "inet",
            "table": table_name,
            "name": "output",
            "type": "filter",
            "hook": "output",
            "prio": 0,
            "policy": "accept",
        }
    })]
}

fn base_nat_chains(table_name: &str) -> Vec<Value> {
    vec![
        json!({
            "chain": {
                "family": "inet",
                "table": table_name,
                "name": "postrouting",
                "type": "nat",
                "hook": "postrouting",
                "prio": 100,
                "policy": "accept",
            },
        }),
        json!({
            "chain": {
                "family": "inet",
                "table": table_name,
                "name": "prerouting",
                "type": "nat",
                "hook": "prerouting",
                "prio": -100,
                "policy": "accept",
            },
        }),
    ]
}

fn stateful_allow_rules(table_name: &str) -> Vec<Value> {
    vec![
        ct_state_accept_rule(table_name, "input"),
        ct_state_accept_rule(table_name, "forward"),
    ]
}

fn loopback_rule(table_name: &str) -> Value {
    json!({
        "rule": {
            "family": "inet",
            "table": table_name,
            "chain": "input",
            "expr": [
                match_iface("iifname", "lo"),
                accept_expr(),
            ],
        }
    })
}

fn policy_service_rules(
    table_name: &str,
    bridge_name: &str,
    policy: &PolicyProfile,
) -> Result<Vec<Value>> {
    let mut rules = Vec::new();

    for service in &policy.services {
        let proto = match service.protocol {
            Protocol::Tcp => ForwardProtocol::Tcp,
            Protocol::Udp => ForwardProtocol::Udp,
            Protocol::Icmp => ForwardProtocol::Icmp,
        };

        let mut expr = vec![match_iface("iifname", bridge_name)];

        match proto {
            ForwardProtocol::Icmp => {
                expr.push(match_l4proto("icmp"));
            }
            ForwardProtocol::Tcp | ForwardProtocol::Udp => {
                let proto_str = proto.as_str();
                expr.push(match_l4proto(proto_str));
                expr.push(match_port(proto_str, "dport", service.port));
            }
        }

        if let Some(ref source) = service.source {
            let net = parse_ipnet(source)?;
            expr.push(match_ip_prefix_expr("saddr", &net));
        }

        expr.push(accept_expr());

        rules.push(json!({
            "rule": {
                "family": "inet",
                "table": table_name,
                "chain": "input",
                "expr": expr,
            }
        }));
    }

    Ok(rules)
}

fn policy_ingress_rules(
    table_name: &str,
    bridge_name: &str,
    policy: &PolicyProfile,
) -> Result<Vec<Value>> {
    let mut rules = Vec::new();

    for cidr in &policy.allowed_ingress_cidrs {
        let net = parse_ipnet(cidr)?;
        let expr = vec![
            match_iface("iifname", bridge_name),
            match_ip_prefix_expr("saddr", &net),
            accept_expr(),
        ];

        rules.push(json!({
            "rule": {
                "family": "inet",
                "table": table_name,
                "chain": "input",
                "expr": expr,
            }
        }));
    }

    Ok(rules)
}

fn policy_egress_rules(
    table_name: &str,
    bridge_name: &str,
    policy: &PolicyProfile,
) -> Result<Vec<Value>> {
    let mut rules = Vec::new();

    for cidr in &policy.allowed_egress_cidrs {
        let net = parse_ipnet(cidr)?;
        let expr = vec![
            match_iface("iifname", bridge_name),
            match_ip_prefix_expr("daddr", &net),
            accept_expr(),
        ];

        rules.push(json!({
            "rule": {
                "family": "inet",
                "table": table_name,
                "chain": "forward",
                "expr": expr,
            }
        }));
    }

    Ok(rules)
}

fn build_nat_rules(
    table_name: &str,
    bridge_name: &str,
    bridge_net: &IpNet,
    gateway: &IpAddr,
    masq_iface: &str,
    forwards: &[ForwardRule],
) -> Vec<Value> {
    let mut rules = Vec::new();

    if !masq_iface.is_empty() {
        rules.push(json!({
            "rule": {
                "family": "inet",
                "table": table_name,
                "chain": "postrouting",
                "expr": [
                    match_iface("oifname", masq_iface),
                    match_ip_prefix_expr("saddr", bridge_net),
                    json!({"masquerade": null}),
                ],
            }
        }));
    }

    for forward in forwards {
        let mut prerouting_expr = vec![
            match_iface("iifname", masq_iface),
            match_l4proto(forward.protocol.as_str()),
            match_port(forward.protocol.as_str(), "dport", forward.public_port),
        ];

        if let Some(addr) = forward.public_addr {
            prerouting_expr.push(match_ip_addr_expr("daddr", &addr));
        }

        prerouting_expr.push(dnat_expr(&forward.dest_addr, forward.dest_port));

        rules.push(json!({
            "rule": {
                "family": "inet",
                "table": table_name,
                "chain": "prerouting",
                "expr": prerouting_expr,
            }
        }));

        let postrouting_expr = vec![
            match_iface("iifname", bridge_name),
            match_iface("oifname", bridge_name),
            match_l4proto(forward.protocol.as_str()),
            match_port(forward.protocol.as_str(), "dport", forward.dest_port),
            match_ip_prefix_expr("saddr", bridge_net),
            match_ip_addr_expr("daddr", &forward.dest_addr),
            snat_expr(gateway),
        ];

        rules.push(json!({
            "rule": {
                "family": "inet",
                "table": table_name,
                "chain": "postrouting",
                "expr": postrouting_expr,
            }
        }));
    }

    rules
}

fn ct_state_accept_rule(table_name: &str, chain: &str) -> Value {
    let expr = vec![
        json!({
            "match": {
                "left": {"ct": {"key": "state"}},
                "op": "in",
                "right": ["established", "related"],
            }
        }),
        accept_expr(),
    ];

    json!({
        "rule": {
            "family": "inet",
            "table": table_name,
            "chain": chain,
            "expr": expr,
        }
    })
}

fn match_iface(key: &str, iface: &str) -> Value {
    json!({
        "match": {
            "left": {"meta": {"key": key}},
            "op": "==",
            "right": iface,
        }
    })
}

fn match_l4proto(proto: &str) -> Value {
    json!({
        "match": {
            "left": {"meta": {"key": "l4proto"}},
            "op": "==",
            "right": proto,
        }
    })
}

fn match_port(proto: &str, field: &str, port: u16) -> Value {
    json!({
        "match": {
            "left": {"payload": {"protocol": proto, "field": field}},
            "op": "==",
            "right": port,
        }
    })
}

fn match_ip_addr_expr(field: &str, ip: &IpAddr) -> Value {
    json!({
        "match": {
            "left": {"payload": {"protocol": ip_protocol(ip), "field": field}},
            "op": "==",
            "right": ip.to_string(),
        }
    })
}

fn match_ip_prefix_expr(field: &str, net: &IpNet) -> Value {
    json!({
        "match": {
            "left": {"payload": {"protocol": ipnet_protocol(net), "field": field}},
            "op": "==",
            "right": {"prefix": {"addr": net.addr().to_string(), "len": net.prefix_len()}},
        }
    })
}

fn dnat_expr(dest: &IpAddr, port: u16) -> Value {
    json!({"dnat": {"addr": dest.to_string(), "port": port}})
}

fn snat_expr(gateway: &IpAddr) -> Value {
    json!({"snat": {"addr": gateway.to_string()}})
}

fn accept_expr() -> Value {
    json!({"accept": null})
}

fn ip_protocol(ip: &IpAddr) -> &'static str {
    match ip {
        IpAddr::V4(_) => "ip",
        IpAddr::V6(_) => "ip6",
    }
}

fn ipnet_protocol(net: &IpNet) -> &'static str {
    match net {
        IpNet::V4(_) => "ip",
        IpNet::V6(_) => "ip6",
    }
}

fn parse_ipnet(value: &str) -> Result<IpNet> {
    value
        .parse()
        .with_context(|| format!("Invalid CIDR '{}'", value))
}

#[derive(Clone, Copy, Debug)]
struct ForwardRule {
    public_addr: Option<IpAddr>,
    public_port: u16,
    protocol: ForwardProtocol,
    dest_addr: IpAddr,
    dest_port: u16,
}

#[derive(Clone, Copy, Debug)]
enum ForwardProtocol {
    Tcp,
    Udp,
    Icmp,
}

impl ForwardProtocol {
    fn from_str(proto: &str) -> Result<Self> {
        match proto.to_ascii_lowercase().as_str() {
            "tcp" => Ok(Self::Tcp),
            "udp" => Ok(Self::Udp),
            "icmp" => Ok(Self::Icmp),
            other => anyhow::bail!("Unsupported protocol '{}'", other),
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Self::Tcp => "tcp",
            Self::Udp => "udp",
            Self::Icmp => "icmp",
        }
    }
}

impl ForwardRule {
    fn parse(public: &str, dest: &str) -> Result<Self> {
        let (addr_part, proto_part) = split_proto(public)?;
        let protocol = ForwardProtocol::from_str(proto_part)?;

        if matches!(protocol, ForwardProtocol::Icmp) {
            anyhow::bail!("ICMP is not supported for port forwards");
        }

        let (public_addr, public_port) = split_host_port(addr_part)?;
        let (dest_addr, dest_port) = split_destination(dest)?;

        Ok(Self {
            public_addr,
            public_port,
            protocol,
            dest_addr,
            dest_port,
        })
    }
}

fn parse_forward_rules(entries: &[(String, String)]) -> Result<Vec<ForwardRule>> {
    entries
        .iter()
        .map(|(public, dest)| ForwardRule::parse(public, dest))
        .collect()
}

fn split_proto(spec: &str) -> Result<(&str, &str)> {
    let idx = spec
        .rfind('/')
        .ok_or_else(|| anyhow!("Missing protocol in {}", spec))?;
    let (addr_part, proto_part) = spec.split_at(idx);
    let proto = &proto_part[1..]; // remove '/'
    if proto.is_empty() {
        anyhow::bail!("Protocol missing in {}", spec);
    }
    Ok((addr_part, proto))
}

fn split_host_port(addr_part: &str) -> Result<(Option<IpAddr>, u16)> {
    let trimmed = addr_part.trim();
    let (host, port_str) = if let Some(idx) = trimmed.rfind(':') {
        (&trimmed[..idx], &trimmed[idx + 1..])
    } else {
        ("", trimmed)
    };

    let port: u16 = port_str.parse().context("Invalid public port")?;

    let host = host.trim();
    let addr = if host.is_empty() || host == "0.0.0.0" {
        None
    } else {
        Some(host.parse().context("Invalid public IP address")?)
    };

    Ok((addr, port))
}

fn split_destination(dest: &str) -> Result<(IpAddr, u16)> {
    let trimmed = dest.trim();
    let (ip_str, port_str) = trimmed
        .rsplit_once(':')
        .ok_or_else(|| anyhow!("Destination must be IP:PORT"))?;
    let ip = ip_str.parse().context("Invalid destination IP address")?;
    let port: u16 = port_str.parse().context("Invalid destination port")?;
    Ok((ip, port))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gw_core::policy::Service;
    use serde_json::Value;

    fn load_ruleset(value: &str) -> Vec<Value> {
        let doc: Value = serde_json::from_str(value).expect("valid JSON");
        doc.get("nftables")
            .and_then(|v| v.as_array())
            .cloned()
            .expect("nftables array")
    }

    fn chain_exprs(entries: &[Value], chain: &str) -> Vec<Value> {
        entries
            .iter()
            .filter_map(|entry| {
                entry
                    .get("rule")
                    .filter(|rule| {
                        rule.get("chain")
                            .and_then(|c| c.as_str())
                            .map(|c| c == chain)
                            .unwrap_or(false)
                    })
                    .and_then(|rule| rule.get("expr"))
                    .and_then(|expr| expr.as_array())
                    .map(|expr| Value::Array(expr.clone()))
            })
            .collect()
    }

    fn expr_has_key(exprs: &Value, key: &str) -> bool {
        exprs
            .as_array()
            .unwrap()
            .iter()
            .any(|expr| expr.get(key).is_some())
    }

    #[test]
    fn complete_ruleset_builds_nat_and_policy_rules() {
        let manager = NftManager::new();

        let policy = PolicyProfile {
            name: "routed-tight".into(),
            description: "Demo profile".into(),
            allowed_ingress_cidrs: vec!["10.33.0.0/24".into()],
            allowed_egress_cidrs: vec!["8.8.8.8/32".into()],
            services: vec![Service {
                protocol: Protocol::Tcp,
                port: 80,
                source: None,
            }],
            default_action: Action::Drop,
        };

        let forwards = vec![(":8080/tcp".to_string(), "10.33.0.10:8080".to_string())];

        let ruleset = manager
            .create_complete_ruleset(
                "gw-test",
                "br-test",
                "10.33.0.0/24",
                "10.33.0.1",
                "eth0",
                &forwards,
                Some(&policy),
            )
            .expect("ruleset generation");

        let nftables = load_ruleset(&ruleset);

        // NAT expectations
        let postrouting = chain_exprs(&nftables, "postrouting");
        assert!(
            postrouting
                .iter()
                .any(|exprs| expr_has_key(exprs, "masquerade"))
        );
        assert!(postrouting.iter().any(|exprs| expr_has_key(exprs, "snat")));

        let prerouting = chain_exprs(&nftables, "prerouting");
        assert!(prerouting.iter().any(|exprs| expr_has_key(exprs, "dnat")));

        // Policy expectations: service rule should match TCP port 80
        let input_chain = chain_exprs(&nftables, "input");
        assert!(input_chain.iter().any(|exprs| {
            exprs.as_array().unwrap().iter().any(|expr| {
                expr.get("match")
                    .and_then(|m| m.get("right"))
                    .and_then(|v| v.as_u64())
                    .map(|port| port == 80)
                    .unwrap_or(false)
            })
        }));
    }
}
