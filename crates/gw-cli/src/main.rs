use clap::{CommandFactory, Parser, Subcommand};
use gw_core::rollback;
use gw_core::{ExecutionContext, Plan, PlanAction, Topology, nft_config_for_table};
use gw_dhcpdns::DnsmasqManager;
use gw_nft::NftManager;
use gw_nl::{AddressManager, BridgeManager};

#[derive(Parser)]
#[command(name = "gwarden")]
#[command(version, about = "Ghost network orchestration", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Network management
    Net {
        #[command(subcommand)]
        action: NetAction,
    },
    /// VM operations
    Vm {
        #[command(subcommand)]
        action: VmAction,
    },
    /// Port forwarding
    Forward {
        #[command(subcommand)]
        action: ForwardAction,
    },
    /// Policy management
    Policy {
        #[command(subcommand)]
        action: PolicyAction,
    },
    /// Terminal UI
    Tui,
    /// Metrics server
    Metrics {
        #[command(subcommand)]
        action: MetricsAction,
    },
    /// Troubleshooting and diagnostics
    Doctor {
        #[command(subcommand)]
        action: Option<DoctorAction>,
    },
    /// Maintainer documentation helpers
    #[command(hide = true)]
    Docs {
        #[command(subcommand)]
        action: DocsAction,
    },
}

#[derive(Subcommand)]
enum NetAction {
    /// Show planned changes without applying
    Plan {
        #[arg(short, long, default_value = "ghostnet.toml")]
        file: String,
    },
    /// Apply network configuration
    Apply {
        #[arg(short, long, default_value = "ghostnet.toml")]
        file: String,
        /// Execute changes on the host; without this flag apply is a dry run
        #[arg(long)]
        commit: bool,
        /// Auto-rollback window in seconds; press ENTER to confirm, 0 disables the wait
        #[arg(long, default_value = "30")]
        confirm: u64,
        /// host:port to probe for connectivity; rollback runs if it is unreachable
        #[arg(long)]
        probe: Option<String>,
        /// Timeout in seconds for the connectivity probe
        #[arg(long, default_value = "3")]
        probe_timeout: u64,
    },
    /// Show current network status
    Status,
    /// Compare desired nftables rules with live system
    Diff {
        #[arg(short, long, default_value = "ghostnet.toml")]
        file: String,
        /// Only diff nftables tables (or networks) matching this name
        #[arg(long)]
        table: Option<String>,
    },
    /// Roll back the last applied configuration snapshot
    Rollback {
        /// Execute the rollback; without this flag only a preview is printed
        #[arg(long)]
        execute: bool,
    },
    /// Show the persisted apply state from the last commit
    State {
        /// Emit the apply state as JSON instead of a human summary
        #[arg(long)]
        json: bool,
    },
    /// Clear the persisted apply state (does not touch live resources)
    #[command(name = "state-clear")]
    StateClear {
        /// Required acknowledgement; clearing state is irreversible
        #[arg(long)]
        confirm: bool,
    },
}

#[derive(Subcommand)]
enum VmAction {
    /// Attach VM to network
    Attach {
        #[arg(long)]
        vm: String,
        #[arg(long)]
        net: String,
        #[arg(long)]
        tap: Option<String>,
    },
    /// List VMs and their network attachments
    List,
}

#[derive(Subcommand)]
enum ForwardAction {
    /// Add port forward
    Add {
        #[arg(long)]
        net: String,
        #[arg(long)]
        public: String,
        #[arg(long)]
        dst: String,
    },
    /// Remove port forward
    Remove {
        #[arg(long)]
        net: String,
        #[arg(long)]
        public: String,
    },
    /// List port forwards
    List,
}

#[derive(Subcommand)]
enum PolicyAction {
    /// Set policy profile for network
    Set {
        #[arg(long)]
        net: String,
        #[arg(long)]
        profile: String,
        #[arg(long, default_value = "ghostnet.toml")]
        file: String,
    },
    /// List available policy profiles
    List,
}

#[derive(Subcommand)]
enum MetricsAction {
    /// Start metrics server
    Serve {
        #[arg(long, default_value = ":9138")]
        addr: String,
    },
}

#[derive(Subcommand)]
enum DoctorAction {
    /// Check nftables/iptables configuration
    Nftables,
    /// Check Docker networking
    Docker,
    /// Check bridge configuration
    Bridges,
    /// Run all diagnostics
    All,
}

#[derive(Subcommand)]
enum DocsAction {
    /// Generate Markdown command reference from clap metadata
    Commands {
        #[arg(short, long, default_value = "docs/reference/commands.md")]
        output: String,
    },
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Net { action } => handle_net_action(action)?,
        Commands::Vm { action } => handle_vm_action(action)?,
        Commands::Forward { action } => handle_forward_action(action)?,
        Commands::Policy { action } => handle_policy_action(action)?,
        Commands::Tui => {
            tokio::runtime::Runtime::new()?.block_on(async { run_tui().await })?;
        }
        Commands::Metrics { action } => {
            tokio::runtime::Runtime::new()?
                .block_on(async { handle_metrics_action(action).await })?;
        }
        Commands::Doctor { action } => {
            tokio::runtime::Runtime::new()?
                .block_on(async { handle_doctor_action(action).await })?;
        }
        Commands::Docs { action } => handle_docs_action(action)?,
    }

    Ok(())
}

fn handle_docs_action(action: DocsAction) -> anyhow::Result<()> {
    match action {
        DocsAction::Commands { output } => {
            let markdown = generate_command_reference();
            std::fs::write(&output, markdown)?;
            println!("Wrote command reference to {}", output);
        }
    }
    Ok(())
}

fn render_help_block(path: &str) -> Option<String> {
    let mut command = Cli::command();
    let mut current = &mut command;

    for segment in path.split_whitespace().skip(1) {
        current = current.find_subcommand_mut(segment)?;
    }

    let mut help = Vec::new();
    current.write_long_help(&mut help).ok()?;
    let help = String::from_utf8(help).ok()?;

    Some(format!("### `{}`\n\n```text\n{}\n```\n", path, help.trim()))
}

fn generate_command_reference() -> String {
    let commands = [
        "gwarden",
        "gwarden net",
        "gwarden net plan",
        "gwarden net apply",
        "gwarden net status",
        "gwarden net diff",
        "gwarden net rollback",
        "gwarden net state",
        "gwarden net state-clear",
        "gwarden vm",
        "gwarden vm attach",
        "gwarden vm list",
        "gwarden forward",
        "gwarden forward add",
        "gwarden forward remove",
        "gwarden forward list",
        "gwarden policy",
        "gwarden policy set",
        "gwarden policy list",
        "gwarden metrics",
        "gwarden metrics serve",
        "gwarden doctor",
        "gwarden doctor nftables",
        "gwarden doctor docker",
        "gwarden doctor bridges",
        "gwarden doctor all",
        "gwarden tui",
    ];

    let mut markdown = String::from(
        "# Commands\n\nThis page is generated from the `gwarden` clap command metadata.\n\n",
    );

    for command in commands {
        if let Some(block) = render_help_block(command) {
            markdown.push_str(&block);
            markdown.push('\n');
        }
    }

    markdown
}

fn handle_net_action(action: NetAction) -> anyhow::Result<()> {
    match action {
        NetAction::Plan { file } => {
            let topology = Topology::from_file(std::path::Path::new(&file))?;
            let plan = Plan::from_topology(&topology)?;
            plan.display();
        }
        NetAction::Apply {
            file,
            commit,
            confirm,
            probe,
            probe_timeout,
        } => {
            // Run async apply
            tokio::runtime::Runtime::new()?.block_on(async {
                apply_network_config(&file, commit, confirm, probe, probe_timeout).await
            })?;
        }
        NetAction::Status => {
            tokio::runtime::Runtime::new()?.block_on(async { show_network_status().await })?;
        }
        NetAction::Diff { file, table } => {
            tokio::runtime::Runtime::new()?
                .block_on(async { diff_network_config(&file, table.as_deref()).await })?;
        }
        NetAction::Rollback { execute } => {
            tokio::runtime::Runtime::new()?
                .block_on(async { run_snapshot_rollback(execute).await })?;
        }
        NetAction::State { json } => {
            show_apply_state(json)?;
        }
        NetAction::StateClear { confirm } => {
            clear_apply_state(confirm)?;
        }
    }
    Ok(())
}

fn show_apply_state(json: bool) -> anyhow::Result<()> {
    let state_path = gw_core::default_apply_state_path()?;
    let Some(state) = gw_core::ApplyState::load_from(&state_path)? else {
        println!("ℹ️  No apply state found at {}", state_path.display());
        return Ok(());
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&state)?);
        return Ok(());
    }

    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    let applied_at = UNIX_EPOCH + Duration::from_secs(state.created_at);
    let age_secs = SystemTime::now()
        .duration_since(applied_at)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs();

    println!("📦 Apply state {}", state.transaction_id);
    println!("📁 {}", state_path.display());
    println!("⏱️  Applied {}s ago", age_secs);
    println!("\nOwned resources ({}):", state.owned_resources.len());
    if state.owned_resources.is_empty() {
        println!("  (none)");
    }
    for resource in &state.owned_resources {
        println!("  - {}", describe_owned_resource(resource));
    }

    Ok(())
}

fn describe_owned_resource(resource: &gw_core::OwnedResource) -> String {
    use gw_core::OwnedResource;
    match resource {
        OwnedResource::Bridge { name } => format!("bridge {}", name),
        OwnedResource::Address { iface, addr } => format!("address {} on {}", addr, iface),
        OwnedResource::NftTable { table } => format!("nftables table {}", table),
        OwnedResource::DnsmasqConfig { path } => format!("dnsmasq config {}", path),
        OwnedResource::Vlan { name } => format!("VLAN {}", name),
    }
}

fn clear_apply_state(confirm: bool) -> anyhow::Result<()> {
    let state_path = gw_core::default_apply_state_path()?;

    if !state_path.exists() {
        println!("ℹ️  No apply state found at {}", state_path.display());
        return Ok(());
    }

    if !confirm {
        println!(
            "⚠️  This clears persisted apply state at {} but does not touch live resources.",
            state_path.display()
        );
        println!("    Re-run with '--confirm' to delete it.");
        return Ok(());
    }

    std::fs::remove_file(&state_path)?;
    println!("🧹 Cleared apply state at {}", state_path.display());
    Ok(())
}

async fn show_network_status() -> anyhow::Result<()> {
    use gw_core::NetworkStatus;
    use gw_dhcpdns::LeaseReader;
    use gw_nft::NftStatusCollector;
    use gw_nl::StatusCollector;

    let mut status = NetworkStatus::new();

    // Collect bridge status
    let bridge_collector = StatusCollector::new().await?;
    status.bridges = bridge_collector.collect_bridge_status().await?;

    // Collect nftables status
    let nft_collector = NftStatusCollector::new();
    status.nftables = nft_collector.collect_table_status().await?;

    // Collect DHCP leases
    let lease_reader = LeaseReader::new();
    status.dhcp_leases = lease_reader.read_default_leases()?;

    // Display status
    status.display();

    Ok(())
}

async fn apply_network_config(
    file: &str,
    commit: bool,
    confirm: u64,
    probe: Option<String>,
    probe_timeout: u64,
) -> anyhow::Result<()> {
    use gw_core::{
        ConflictDetector, ExecutionContext, Plan, PlanAction, RollbackManager, Topology,
        TopologyValidator,
    };
    use gw_dhcpdns::DnsmasqManager;
    use gw_nft::NftManager;
    use gw_nl::{AddressManager, BridgeManager};

    println!("🚀 Loading topology from {}", file);
    let topology = Topology::from_file(std::path::Path::new(file))?;

    // Validate topology
    println!("🔍 Validating topology...");
    let validator = TopologyValidator::new(&topology);
    let validation_warnings = validator.validate()?;

    if !validation_warnings.is_empty() {
        println!("\n⚠️  Validation warnings/errors found:\n");
        let mut has_errors = false;
        for warning in &validation_warnings {
            warning.display();
            if warning.is_error() {
                has_errors = true;
            }
            println!();
        }

        if has_errors {
            anyhow::bail!("Topology validation failed. Please fix the errors above.");
        }

        if !commit {
            println!("⚠️  Warnings found but will not block apply in commit mode.\n");
        }
    } else {
        println!("✅ Topology validation passed\n");
    }

    // Check for conflicts
    println!("🔍 Checking for system conflicts...");
    let detector = ConflictDetector::new();
    let conflict_report = detector.detect_all().await?;
    conflict_report.display();

    if conflict_report.has_errors() && !commit {
        println!(
            "\n❌ Found critical conflicts. Fix them before applying, or use --commit --force to override"
        );
        return Ok(());
    }

    println!("\n📋 Generating plan...");
    let plan = Plan::from_topology(&topology)?;
    plan.display();

    if !commit {
        println!("\n⚠️  Dry run mode. Use --commit to apply changes.");
        return Ok(());
    }

    println!("\n⚡ Applying configuration...");

    // Create managers
    let bridge_mgr = BridgeManager::new().await?;
    let addr_mgr = AddressManager::new().await?;
    let nft_mgr = NftManager::new();
    let dnsmasq_mgr = DnsmasqManager::new();
    let vlan_mgr = gw_nl::VlanManager::new().await?;

    let mut context = ExecutionContext::new(true);
    context.attach_plan(plan.clone());

    let profiles = gw_core::ProfileLoader::new().load_default_profiles();

    // Execute plan actions
    for (i, action) in plan.actions.iter().enumerate() {
        println!("\n[{}/{}] {}", i + 1, plan.actions.len(), action);

        match action {
            PlanAction::CreateBridge { name, cidr } => {
                bridge_mgr.create_bridge(name).await?;
                if let Some(cidr_str) = cidr {
                    // Extract gateway IP from CIDR for address assignment
                    let gw_ip = extract_gateway_ip(cidr_str, &topology)?;
                    addr_mgr.add_address(name, &gw_ip).await?;
                }
                context.record_action(action.clone());
            }
            PlanAction::AddAddress { iface, addr } => {
                addr_mgr.add_address(iface, addr).await?;
                context.record_action(action.clone());
            }
            PlanAction::EnableForwarding { iface } => {
                bridge_mgr.enable_forwarding(iface).await?;
                context.record_action(action.clone());
            }
            PlanAction::CreateNftRuleset { table, .. } => {
                if let Some(generated) = generate_ruleset(&nft_mgr, &topology, table, &profiles)? {
                    if let Some(policy_name) = &generated.policy_loaded {
                        println!("   📜 Loaded policy profile: {}", policy_name);
                    }
                    if let Some(missing) = &generated.policy_missing {
                        eprintln!("   ⚠️  Policy profile '{}' not found", missing);
                    }

                    let snapshot = nft_mgr.apply_ruleset(table, &generated.ruleset).await?;
                    context.record_nft_snapshot(table.clone(), snapshot);
                    context.record_action(action.clone());
                }
            }
            PlanAction::StartDnsmasq { config_path } => {
                // Generate and write dnsmasq config
                if let Some(dns_config) = get_dns_config(&topology, config_path)? {
                    let config = dnsmasq_mgr.generate_config(
                        &dns_config.bridge,
                        &dns_config.cidr,
                        &dns_config.zones,
                    )?;
                    dnsmasq_mgr.write_config(config_path, &config)?;
                    dnsmasq_mgr.restart().await?;
                    context.record_action(action.clone());
                }
            }
            PlanAction::CreateVlan {
                parent,
                vlan_id,
                name,
            } => {
                vlan_mgr.create_vlan(parent, *vlan_id, name).await?;
                context.record_action(action.clone());
            }
            PlanAction::AttachVlanToBridge { vlan, bridge } => {
                vlan_mgr.attach_vlan_to_bridge(vlan, bridge).await?;
                context.record_action(action.clone());
            }
        }
    }

    println!("\n✅ Configuration applied successfully!");

    // Share one transaction ID across the rollback snapshot and apply state so
    // `gwarden net rollback` and `gwarden net state` reference the same apply.
    let transaction_id = gw_core::new_transaction_id();

    let record = context.to_rollback_record(transaction_id.clone());
    let record_path = rollback::save_record(&record)?;
    println!(
        "💾 Saved rollback snapshot {} to {}",
        transaction_id,
        record_path.display()
    );

    let apply_state = gw_core::ApplyState::from_plan(
        transaction_id.clone(),
        plan.clone(),
        &context.actions_completed,
    );
    let state_path = gw_core::default_apply_state_path()?;
    apply_state.save_to(&state_path)?;
    println!(
        "💾 Saved apply state {} to {}",
        apply_state.transaction_id,
        state_path.display()
    );

    let rollback_mgr = RollbackManager::new(confirm);

    if let Some(target) = probe.as_ref() {
        let timeout_secs = probe_timeout.max(1);
        println!(
            "\n🔍 Probing connectivity to {} ({}s timeout)...",
            target, timeout_secs
        );

        let reachable = rollback_mgr
            .check_tcp_connectivity(target, timeout_secs)
            .await?;

        if reachable {
            println!("✅ Connectivity probe succeeded");
        } else {
            println!("❌ Connectivity probe failed; rolling back changes");
            execute_rollback(
                &context,
                &bridge_mgr,
                &addr_mgr,
                &nft_mgr,
                &dnsmasq_mgr,
                &vlan_mgr,
            )
            .await?;
            rollback::clear_record()?;
            anyhow::bail!("Connectivity probe failed for {}", target);
        }
    }

    // Handle rollback confirmation
    if confirm > 0 {
        let confirmed = rollback_mgr.wait_for_confirmation().await?;

        if !confirmed {
            // User didn't confirm - rollback
            println!("\n🔄 Rolling back configuration...");

            // Execute rollback using managers
            execute_rollback(
                &context,
                &bridge_mgr,
                &addr_mgr,
                &nft_mgr,
                &dnsmasq_mgr,
                &vlan_mgr,
            )
            .await?;
            rollback::clear_record()?;

            anyhow::bail!("Configuration rolled back due to timeout");
        }
    }

    Ok(())
}

fn table_matches_filter(filter: &str, table: &str) -> bool {
    let filter = filter.trim();
    if filter.is_empty() {
        return true;
    }

    let filter_lower = filter.to_ascii_lowercase();
    let table_lower = table.to_ascii_lowercase();
    table_lower.contains(&filter_lower)
}

fn print_diff(diff_text: &str) {
    println!("   --- diff ---");
    for line in diff_text.lines() {
        println!("   {}", line);
    }
}

async fn diff_network_config(file: &str, table_filter: Option<&str>) -> anyhow::Result<()> {
    use std::path::Path;

    println!("🔍 Loading topology from {}", file);
    let topology = Topology::from_file(Path::new(file))?;
    let plan = Plan::from_topology(&topology)?;
    let nft_mgr = NftManager::new();
    let profiles = gw_core::ProfileLoader::new().load_default_profiles();

    let filter_owned = table_filter.map(|f| f.to_string());
    let mut matched_any = false;

    for action in &plan.actions {
        if let PlanAction::CreateNftRuleset { table, .. } = action {
            if let Some(filter) = filter_owned.as_deref()
                && !table_matches_filter(filter, table)
            {
                // Allow matching on network name as well
                if let Some(config) = nft_config_for_table(&topology, table) {
                    if !table_matches_filter(filter, &config.network_name) {
                        continue;
                    }
                } else {
                    continue;
                }
            }

            if let Some(generated) = generate_ruleset(&nft_mgr, &topology, table, &profiles)? {
                matched_any = true;
                println!("\n=== Table {} (network {}) ===", table, generated.network);
                if let Some(policy_name) = &generated.policy_loaded {
                    println!("   📜 Policy: {}", policy_name);
                } else if let Some(missing) = &generated.policy_missing {
                    println!(
                        "   ⚠️  Policy '{}' not found; diff computed without policy",
                        missing
                    );
                }

                let diff = nft_mgr.diff_ruleset(table, &generated.ruleset).await?;
                if diff.matches {
                    println!("✅ Table is in sync with desired ruleset.");
                } else {
                    if diff.current_exists {
                        println!("❌ Drift detected between desired and live rules.");
                    } else {
                        println!("❌ Table is missing from the system.");
                    }

                    if let Some(diff_text) = diff.diff.as_ref() {
                        print_diff(diff_text);
                    }
                }
            }
        }
    }

    if !matched_any {
        if let Some(filter) = table_filter {
            println!(
                "No nftables entries matched filter '{}' in plan {}.",
                filter, file
            );
        } else {
            println!("No nftables tables found in the generated plan.");
        }
    }

    Ok(())
}

/// Execute rollback by deleting all created resources in reverse order
async fn execute_rollback(
    context: &ExecutionContext,
    bridge_mgr: &BridgeManager,
    addr_mgr: &AddressManager,
    nft_mgr: &NftManager,
    dnsmasq_mgr: &DnsmasqManager,
    vlan_mgr: &gw_nl::VlanManager,
) -> anyhow::Result<()> {
    use gw_core::RollbackOp;

    for op in context.rollback_operations() {
        match op {
            RollbackOp::DeleteBridge { name } => {
                println!("  ⏪ Deleting bridge: {}", name);
                if let Err(e) = bridge_mgr.delete_bridge(&name).await {
                    eprintln!("     ⚠️  Failed to delete bridge {}: {}", name, e);
                }
            }
            RollbackOp::RemoveAddress { iface, addr } => {
                println!("  ⏪ Removing address {} from {}", addr, iface);
                if let Err(e) = addr_mgr.delete_address(&iface, &addr).await {
                    eprintln!("     ⚠️  Failed to remove address: {}", e);
                }
            }
            RollbackOp::RestoreNft { table, snapshot } => {
                let snapshot_ref = snapshot.as_deref();
                let action_desc = if snapshot_ref.is_some() {
                    "Restoring"
                } else {
                    "Deleting"
                };
                println!("  ⏪ {} nftables table: {}", action_desc, table);
                if let Err(e) = nft_mgr
                    .restore_table_from_snapshot(&table, snapshot_ref)
                    .await
                {
                    eprintln!("     ⚠️  Failed to revert nftables table: {}", e);
                }
            }
            RollbackOp::DeleteDnsmasqConfig { path } => {
                println!("  ⏪ Deleting dnsmasq config: {}", path);
                if let Err(e) = dnsmasq_mgr.delete_config(&path) {
                    eprintln!("     ⚠️  Failed to delete config: {}", e);
                }
                // Restart dnsmasq to apply changes
                if let Err(e) = dnsmasq_mgr.restart().await {
                    eprintln!("     ⚠️  Failed to restart dnsmasq: {}", e);
                }
            }
            RollbackOp::DeleteVlan { name } => {
                println!("  ⏪ Deleting VLAN: {}", name);
                if let Err(e) = vlan_mgr.delete_vlan(&name).await {
                    eprintln!("     ⚠️  Failed to delete VLAN: {}", e);
                }
            }
        }
    }

    println!("✅ Rollback completed");
    Ok(())
}

// Helper to extract gateway IP from topology
fn extract_gateway_ip(cidr: &str, topology: &Topology) -> anyhow::Result<String> {
    // Find the network that uses this CIDR and get its gateway IP
    for network in topology.networks.values() {
        if let gw_core::Network::Routed(routed) = network
            && routed.cidr == cidr
        {
            return Ok(format!(
                "{}/{}",
                routed.gw_ip,
                cidr.split('/').nth(1).unwrap()
            ));
        }
    }
    anyhow::bail!("Could not find gateway IP for CIDR {}", cidr)
}

// Helper to get NFT config for a table
struct GeneratedRuleset {
    network: String,
    ruleset: String,
    policy_loaded: Option<String>,
    policy_missing: Option<String>,
}

fn generate_ruleset(
    nft_mgr: &NftManager,
    topology: &Topology,
    table: &str,
    profiles: &std::collections::HashMap<String, gw_core::PolicyProfile>,
) -> anyhow::Result<Option<GeneratedRuleset>> {
    let Some(config) = nft_config_for_table(topology, table) else {
        return Ok(None);
    };

    let bridge_name = format!("br-{}", config.network_name);

    let mut policy_loaded = None;
    let mut policy_missing = None;
    let policy = config.policy_profile.as_ref().and_then(|name| {
        if let Some(profile) = profiles.get(name) {
            policy_loaded = Some(name.clone());
            Some(profile)
        } else {
            policy_missing = Some(name.clone());
            None
        }
    });

    let ruleset = nft_mgr.create_complete_ruleset(
        table,
        &bridge_name,
        &config.cidr,
        &config.gateway_ip,
        &config.masq_iface,
        &config.forwards,
        policy,
    )?;

    Ok(Some(GeneratedRuleset {
        network: config.network_name,
        ruleset,
        policy_loaded,
        policy_missing,
    }))
}

// Helper to get DNS config
struct DnsConfig {
    bridge: String,
    cidr: String,
    zones: Vec<String>,
}

fn get_dns_config(topology: &Topology, config_path: &str) -> anyhow::Result<Option<DnsConfig>> {
    // Extract network name from config path
    // e.g., "/etc/dnsmasq.d/gw-nat_dev.conf" -> "nat_dev"
    for (name, network) in &topology.networks {
        if config_path.contains(name)
            && let gw_core::Network::Routed(routed) = network
            && routed.dhcp
        {
            let zones = if let Some(dns) = &routed.dns {
                dns.zones.clone()
            } else {
                vec![]
            };

            return Ok(Some(DnsConfig {
                bridge: format!("br-{}", name),
                cidr: routed.cidr.clone(),
                zones,
            }));
        }
    }
    Ok(None)
}

async fn run_snapshot_rollback(execute: bool) -> anyhow::Result<()> {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use gw_dhcpdns::DnsmasqManager;
    use gw_nft::NftManager;
    use gw_nl::{AddressManager, BridgeManager};

    let record_path = rollback::default_record_path()?;
    let Some(record) = rollback::load_record()? else {
        println!(
            "ℹ️  No rollback snapshot found at {}",
            record_path.display()
        );
        return Ok(());
    };

    let snapshot_time = UNIX_EPOCH + Duration::from_secs(record.created_at);
    let age_secs = SystemTime::now()
        .duration_since(snapshot_time)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs();

    let transaction_label = if record.transaction_id.is_empty() {
        "(legacy snapshot)".to_string()
    } else {
        record.transaction_id.clone()
    };
    println!(
        "📦 Rollback snapshot {} ({} actions) saved {}s ago",
        transaction_label,
        record.actions.len(),
        age_secs
    );
    println!("📁 {}", record_path.display());

    // Preview the exact rollback operations, in execution order, before running them.
    let context = ExecutionContext::from_rollback_record(record);
    let operations = context.rollback_operations();
    if operations.is_empty() {
        println!("\nℹ️  Snapshot has no reversible operations.");
        return Ok(());
    }

    println!("\nThe following operations will run, in order:");
    for op in &operations {
        println!("  - {}", describe_rollback_op(op));
    }

    if !execute {
        println!("\nRun with '--execute' to apply this rollback.");
        return Ok(());
    }

    println!("\n🔄 Executing rollback from snapshot...");

    let bridge_mgr = BridgeManager::new().await?;
    let addr_mgr = AddressManager::new().await?;
    let nft_mgr = NftManager::new();
    let dnsmasq_mgr = DnsmasqManager::new();
    let vlan_mgr = gw_nl::VlanManager::new().await?;

    execute_rollback(
        &context,
        &bridge_mgr,
        &addr_mgr,
        &nft_mgr,
        &dnsmasq_mgr,
        &vlan_mgr,
    )
    .await?;
    rollback::clear_record()?;
    println!("✅ Snapshot rollback completed");

    Ok(())
}

/// Render a single rollback operation as a human-readable preview line.
fn describe_rollback_op(op: &gw_core::RollbackOp) -> String {
    use gw_core::RollbackOp;
    match op {
        RollbackOp::DeleteBridge { name } => format!("delete bridge {}", name),
        RollbackOp::RemoveAddress { iface, addr } => {
            format!("remove address {} from {}", addr, iface)
        }
        RollbackOp::RestoreNft { table, snapshot } => {
            if snapshot.is_some() {
                format!("restore nftables table {}", table)
            } else {
                format!("delete nftables table {}", table)
            }
        }
        RollbackOp::DeleteDnsmasqConfig { path } => format!("delete dnsmasq config {}", path),
        RollbackOp::DeleteVlan { name } => format!("delete VLAN {}", name),
    }
}

fn handle_vm_action(action: VmAction) -> anyhow::Result<()> {
    match action {
        VmAction::Attach { vm, net, tap } => {
            tokio::runtime::Runtime::new()?
                .block_on(async { attach_vm_to_network(&vm, &net, tap.as_deref()).await })?;
        }
        VmAction::List => {
            tokio::runtime::Runtime::new()?.block_on(async { list_vms().await })?;
        }
    }
    Ok(())
}

async fn list_vms() -> anyhow::Result<()> {
    use gw_libvirt::LibvirtManager;

    let mgr = LibvirtManager::new();
    let vms = mgr.list_vms().await?;

    if vms.is_empty() {
        println!("No VMs found");
        return Ok(());
    }

    println!("VMs ({}):\n", vms.len());
    for vm in vms {
        let id_str = vm
            .id
            .map(|i| i.to_string())
            .unwrap_or_else(|| "-".to_string());
        println!("  {} {} [{}]", id_str, vm.name, vm.state);
        if !vm.interfaces.is_empty() {
            println!("    Interfaces: {}", vm.interfaces.join(", "));
        }
    }

    Ok(())
}

async fn attach_vm_to_network(vm: &str, bridge: &str, tap: Option<&str>) -> anyhow::Result<()> {
    use gw_libvirt::LibvirtManager;

    let mgr = LibvirtManager::new();

    // Convert network name to bridge name (e.g., "nat_dev" -> "br-nat_dev")
    let bridge_name = if bridge.starts_with("br-") {
        bridge.to_string()
    } else {
        format!("br-{}", bridge)
    };

    mgr.attach_vm_to_bridge(vm, &bridge_name, tap).await?;

    Ok(())
}

fn default_topology_path() -> std::path::PathBuf {
    for candidate in ["ghostnet.toml", "ghostnet.yaml", "ghostnet.yml"] {
        let path = std::path::PathBuf::from(candidate);
        if path.exists() {
            return path;
        }
    }

    std::path::PathBuf::from("ghostnet.toml")
}

fn handle_forward_action(action: ForwardAction) -> anyhow::Result<()> {
    use gw_core::Topology;

    let topology_path = default_topology_path();

    match action {
        ForwardAction::Add { net, public, dst } => {
            // Load existing topology
            let mut topology = if topology_path.exists() {
                Topology::from_file(&topology_path)?
            } else {
                anyhow::bail!("Topology file not found: {}", topology_path.display());
            };

            // Find the network
            let network = topology
                .networks
                .get_mut(&net)
                .ok_or_else(|| anyhow::anyhow!("Network '{}' not found in topology", net))?;

            // Add forward to the network
            match network {
                gw_core::Network::Routed(routed) => {
                    // Check if forward already exists
                    if routed.forwards.iter().any(|f| f.public == public) {
                        anyhow::bail!("Forward for '{}' already exists", public);
                    }

                    routed.forwards.push(gw_core::PortForward {
                        public: public.clone(),
                        dst: dst.clone(),
                    });

                    topology.write_file(&topology_path)?;

                    println!("✅ Added port forward: {} -> {}", public, dst);
                    println!("   Run 'gwarden net apply --commit' to activate");
                }
                _ => {
                    anyhow::bail!(
                        "Network '{}' is not a routed network (only routed networks support port forwards)",
                        net
                    );
                }
            }
        }
        ForwardAction::Remove { net, public } => {
            // Load existing topology
            let mut topology = if topology_path.exists() {
                Topology::from_file(&topology_path)?
            } else {
                anyhow::bail!("Topology file not found: {}", topology_path.display());
            };

            // Find the network
            let network = topology
                .networks
                .get_mut(&net)
                .ok_or_else(|| anyhow::anyhow!("Network '{}' not found in topology", net))?;

            // Remove forward from the network
            match network {
                gw_core::Network::Routed(routed) => {
                    let original_len = routed.forwards.len();
                    routed.forwards.retain(|f| f.public != public);

                    if routed.forwards.len() == original_len {
                        anyhow::bail!("Forward for '{}' not found", public);
                    }

                    topology.write_file(&topology_path)?;

                    println!("✅ Removed port forward: {}", public);
                    println!("   Run 'gwarden net apply --commit' to activate");
                }
                _ => {
                    anyhow::bail!("Network '{}' is not a routed network", net);
                }
            }
        }
        ForwardAction::List => {
            // Load existing topology
            let topology = if topology_path.exists() {
                Topology::from_file(&topology_path)?
            } else {
                anyhow::bail!("Topology file not found: {}", topology_path.display());
            };

            println!("📋 Port Forwards:\n");
            let mut found_any = false;

            for (net_name, network) in &topology.networks {
                if let gw_core::Network::Routed(routed) = network
                    && !routed.forwards.is_empty()
                {
                    found_any = true;
                    println!("Network: {}", net_name);
                    for forward in &routed.forwards {
                        println!("  {} -> {}", forward.public, forward.dst);
                    }
                    println!();
                }
            }

            if !found_any {
                println!("  (no port forwards configured)");
            }
        }
    }
    Ok(())
}

fn handle_policy_action(action: PolicyAction) -> anyhow::Result<()> {
    match action {
        PolicyAction::Set { net, profile, file } => {
            use gw_core::{Network, ProfileLoader, Topology};

            let path = std::path::Path::new(&file);
            if !path.exists() {
                anyhow::bail!("Topology file not found: {}", path.display());
            }

            let mut topology = Topology::from_file(path)?;

            let profile_loader = ProfileLoader::new();
            let profiles = profile_loader.load_default_profiles();

            let profile_value = if matches!(profile.as_str(), "none" | "clear" | "off") {
                None
            } else {
                match profiles.get(&profile) {
                    Some(p) => Some(p.name.clone()),
                    None => {
                        anyhow::bail!("Policy profile '{}' not found", profile);
                    }
                }
            };

            let network = topology
                .networks
                .get_mut(&net)
                .ok_or_else(|| anyhow::anyhow!("Network '{}' not found in topology", net))?;

            match network {
                Network::Routed(routed) => {
                    routed.policy_profile = profile_value.clone();
                }
                Network::Bridge(bridge) => {
                    bridge.policy_profile = profile_value.clone();
                }
                Network::Vxlan(_) => {
                    anyhow::bail!("Policy profiles are not currently supported for VXLAN networks");
                }
            }

            topology.write_file(path)?;

            match profile_value {
                Some(name) => {
                    println!(
                        "✅ Updated policy for network '{}' to '{}' in {}",
                        net,
                        name,
                        path.display()
                    );
                }
                None => {
                    println!(
                        "✅ Cleared policy for network '{}' in {}",
                        net,
                        path.display()
                    );
                }
            }
            println!("   Run 'gwarden net apply --commit' to activate the changes.");
        }
        PolicyAction::List => {
            list_policy_profiles()?;
        }
    }
    Ok(())
}

fn list_policy_profiles() -> anyhow::Result<()> {
    use gw_core::ProfileLoader;

    let loader = ProfileLoader::new();
    let profiles = loader.load_default_profiles();

    if profiles.is_empty() {
        println!("No policy profiles found");
        println!("Add profiles to examples/policies/ or /etc/gwarden/policies/");
        return Ok(());
    }

    println!("Available policy profiles ({}):\n", profiles.len());
    for (name, profile) in profiles {
        println!("  • {} - {}", name, profile.description);
        println!("    Default action: {:?}", profile.default_action);
        println!("    Services: {}", profile.services.len());
    }

    Ok(())
}

async fn run_tui() -> anyhow::Result<()> {
    use gw_tui::TuiApp;

    let mut app = TuiApp::new();
    app.run().await?;

    Ok(())
}

async fn handle_metrics_action(action: MetricsAction) -> anyhow::Result<()> {
    match action {
        MetricsAction::Serve { addr } => {
            use gw_metrics::{MetricsCollector, MetricsServer};

            // Parse port from address
            let port: u16 = addr.trim_start_matches(':').parse().unwrap_or(9138);

            println!("🚀 Starting metrics server on port {}...", port);

            // Create collector
            let collector = MetricsCollector::new()?;

            // Create and start server
            let server = MetricsServer::new(collector, port);
            server.serve().await?;
        }
    }
    Ok(())
}

async fn handle_doctor_action(action: Option<DoctorAction>) -> anyhow::Result<()> {
    use gw_troubleshoot::Troubleshooter;

    let troubleshooter = Troubleshooter::new();

    match action {
        Some(DoctorAction::Nftables) => {
            println!("🔍 Checking nftables/iptables configuration...\n");
            let results = troubleshooter.check_nftables().await?;
            for result in results {
                result.display();
            }
        }
        Some(DoctorAction::Docker) => {
            println!("🔍 Checking Docker networking...\n");
            let results = troubleshooter.check_docker().await?;
            for result in results {
                result.display();
            }
        }
        Some(DoctorAction::Bridges) => {
            println!("🔍 Checking bridge configuration...\n");
            let results = troubleshooter.check_bridges().await?;
            for result in results {
                result.display();
            }
        }
        Some(DoctorAction::All) | None => {
            println!("🩺 Running comprehensive network diagnostics...\n");
            let report = troubleshooter.run_all().await?;
            report.display();
        }
    }

    Ok(())
}
