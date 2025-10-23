use clap::{Parser, Subcommand};
use gw_core::{Plan, Topology, ExecutionContext};
use gw_nl::{BridgeManager, AddressManager};
use gw_nft::NftManager;
use gw_dhcpdns::DnsmasqManager;
use tracing_subscriber;

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
}

#[derive(Subcommand)]
enum NetAction {
    /// Show planned changes without applying
    Plan {
        #[arg(short, long, default_value = "ghostnet.yaml")]
        file: String,
    },
    /// Apply network configuration
    Apply {
        #[arg(short, long, default_value = "ghostnet.yaml")]
        file: String,
        #[arg(long)]
        commit: bool,
        #[arg(long, default_value = "30")]
        confirm: u64,
    },
    /// Show current network status
    Status,
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

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Net { action } => handle_net_action(action)?,
        Commands::Vm { action } => handle_vm_action(action)?,
        Commands::Forward { action } => handle_forward_action(action)?,
        Commands::Policy { action } => handle_policy_action(action)?,
        Commands::Tui => {
            tokio::runtime::Runtime::new()?.block_on(async {
                run_tui().await
            })?;
        }
        Commands::Metrics { action } => {
            tokio::runtime::Runtime::new()?.block_on(async {
                handle_metrics_action(action).await
            })?;
        }
        Commands::Doctor { action } => {
            tokio::runtime::Runtime::new()?.block_on(async {
                handle_doctor_action(action).await
            })?;
        }
    }

    Ok(())
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
        } => {
            // Run async apply
            tokio::runtime::Runtime::new()?.block_on(async {
                apply_network_config(&file, commit, confirm).await
            })?;
        }
        NetAction::Status => {
            tokio::runtime::Runtime::new()?.block_on(async {
                show_network_status().await
            })?;
        }
    }
    Ok(())
}

async fn show_network_status() -> anyhow::Result<()> {
    use gw_core::NetworkStatus;
    use gw_nl::StatusCollector;
    use gw_nft::NftStatusCollector;
    use gw_dhcpdns::LeaseReader;

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

async fn apply_network_config(file: &str, commit: bool, confirm: u64) -> anyhow::Result<()> {
    use gw_core::{ExecutionContext, Topology, Plan, PlanAction, ConflictDetector, RollbackManager, TopologyValidator};
    use gw_nl::{BridgeManager, AddressManager};
    use gw_nft::NftManager;
    use gw_dhcpdns::DnsmasqManager;

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
        println!("\n❌ Found critical conflicts. Fix them before applying, or use --commit --force to override");
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
            PlanAction::CreateNftRuleset { table, rules: _ } => {
                // Generate and apply nftables ruleset
                if let Some(nft_config) = get_nft_config(&topology, table)? {
                    // Load policy profile if specified
                    let policy = if let Some(profile_name) = &nft_config.policy_profile {
                        let profile_loader = gw_core::ProfileLoader::new();
                        let profiles = profile_loader.load_default_profiles();

                        if let Some(p) = profiles.get(profile_name) {
                            println!("   📜 Loaded policy profile: {}", profile_name);
                            Some(p.clone())
                        } else {
                            eprintln!("   ⚠️  Policy profile '{}' not found", profile_name);
                            None
                        }
                    } else {
                        None
                    };

                    // Generate complete ruleset with NAT + policy filtering
                    let bridge_name = format!("br-{}", nft_config.network_name);
                    let ruleset = nft_mgr.create_complete_ruleset(
                        table,
                        &bridge_name,
                        &nft_config.cidr,
                        &nft_config.masq_iface,
                        &nft_config.forwards,
                        policy.as_ref(),
                    )?;
                    nft_mgr.apply_ruleset(&ruleset).await?;
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
            PlanAction::CreateVlan { parent, vlan_id, name } => {
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

    // Handle rollback confirmation
    if confirm > 0 {
        let rollback_mgr = RollbackManager::new(confirm);
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
            ).await?;

            anyhow::bail!("Configuration rolled back due to timeout");
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
) -> anyhow::Result<()> {
    use gw_core::planner::Action as PlanAction;

    // Process actions in reverse order
    for action in context.actions_completed.iter().rev() {
        match action {
            PlanAction::CreateBridge { name, .. } => {
                println!("  ⏪ Deleting bridge: {}", name);
                if let Err(e) = bridge_mgr.delete_bridge(name).await {
                    eprintln!("     ⚠️  Failed to delete bridge {}: {}", name, e);
                }
            }
            PlanAction::AddAddress { iface, addr } => {
                println!("  ⏪ Removing address {} from {}", addr, iface);
                if let Err(e) = addr_mgr.delete_address(iface, addr).await {
                    eprintln!("     ⚠️  Failed to remove address: {}", e);
                }
            }
            PlanAction::CreateNftRuleset { table, .. } => {
                println!("  ⏪ Deleting nftables table: {}", table);
                if let Err(e) = nft_mgr.delete_table(table).await {
                    eprintln!("     ⚠️  Failed to delete nftables table: {}", e);
                }
            }
            PlanAction::StartDnsmasq { config_path } => {
                println!("  ⏪ Deleting dnsmasq config: {}", config_path);
                if let Err(e) = dnsmasq_mgr.delete_config(config_path) {
                    eprintln!("     ⚠️  Failed to delete config: {}", e);
                }
                // Restart dnsmasq to apply changes
                if let Err(e) = dnsmasq_mgr.restart().await {
                    eprintln!("     ⚠️  Failed to restart dnsmasq: {}", e);
                }
            }
            PlanAction::CreateVlan { name, .. } => {
                println!("  ⏪ Deleting VLAN: {}", name);
                // VLANs are deleted when the bridge is deleted or can be deleted explicitly
                if let Err(e) = bridge_mgr.delete_bridge(name).await {
                    eprintln!("     ⚠️  Failed to delete VLAN: {}", e);
                }
            }
            _ => {
                // Other actions like EnableForwarding don't need explicit rollback
            }
        }
    }

    println!("✅ Rollback completed");
    Ok(())
}

// Helper to extract gateway IP from topology
fn extract_gateway_ip(cidr: &str, topology: &Topology) -> anyhow::Result<String> {
    // Find the network that uses this CIDR and get its gateway IP
    for (_name, network) in &topology.networks {
        if let gw_core::Network::Routed(routed) = network {
            if routed.cidr == cidr {
                return Ok(format!("{}/{}", routed.gw_ip, cidr.split('/').nth(1).unwrap()));
            }
        }
    }
    anyhow::bail!("Could not find gateway IP for CIDR {}", cidr)
}

// Helper to get NFT config for a table
struct NftConfig {
    network_name: String,
    cidr: String,
    masq_iface: String,
    forwards: Vec<(String, String)>,
    policy_profile: Option<String>,
}

fn get_nft_config(topology: &Topology, _table_name: &str) -> anyhow::Result<Option<NftConfig>> {
    // Extract network name from table name (e.g., "gw" -> look for routed networks)
    for (name, network) in &topology.networks {
        if let gw_core::Network::Routed(routed) = network {
            if let Some(masq_out) = &routed.masq_out {
                let forwards: Vec<(String, String)> = routed
                    .forwards
                    .iter()
                    .map(|f| (f.public.clone(), f.dst.clone()))
                    .collect();

                return Ok(Some(NftConfig {
                    network_name: name.clone(),
                    cidr: routed.cidr.clone(),
                    masq_iface: masq_out.clone(),
                    forwards,
                    policy_profile: routed.policy_profile.clone(),
                }));
            }
        }
    }
    Ok(None)
}

// Helper to get DNS config
struct DnsConfig {
    bridge: String,
    cidr: String,
    zones: Vec<String>,
}

fn get_dns_config(
    topology: &Topology,
    config_path: &str,
) -> anyhow::Result<Option<DnsConfig>> {
    // Extract network name from config path
    // e.g., "/etc/dnsmasq.d/gw-nat_dev.conf" -> "nat_dev"
    for (name, network) in &topology.networks {
        if config_path.contains(name) {
            if let gw_core::Network::Routed(routed) = network {
                if routed.dhcp {
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
        }
    }
    Ok(None)
}

fn handle_vm_action(action: VmAction) -> anyhow::Result<()> {
    match action {
        VmAction::Attach { vm, net, tap } => {
            tokio::runtime::Runtime::new()?.block_on(async {
                attach_vm_to_network(&vm, &net, tap.as_deref()).await
            })?;
        }
        VmAction::List => {
            tokio::runtime::Runtime::new()?.block_on(async {
                list_vms().await
            })?;
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
        let id_str = vm.id.map(|i| i.to_string()).unwrap_or_else(|| "-".to_string());
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

fn handle_forward_action(action: ForwardAction) -> anyhow::Result<()> {
    use gw_core::Topology;

    let topology_path = std::path::Path::new("ghostnet.yaml");

    match action {
        ForwardAction::Add { net, public, dst } => {
            // Load existing topology
            let mut topology = if topology_path.exists() {
                Topology::from_file(topology_path)?
            } else {
                anyhow::bail!("Topology file not found: {}", topology_path.display());
            };

            // Find the network
            let network = topology.networks.get_mut(&net)
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

                    // Save topology
                    let yaml = serde_yaml::to_string(&topology)?;
                    std::fs::write(topology_path, yaml)?;

                    println!("✅ Added port forward: {} -> {}", public, dst);
                    println!("   Run 'gwarden net apply --commit' to activate");
                }
                _ => {
                    anyhow::bail!("Network '{}' is not a routed network (only routed networks support port forwards)", net);
                }
            }
        }
        ForwardAction::Remove { net, public } => {
            // Load existing topology
            let mut topology = if topology_path.exists() {
                Topology::from_file(topology_path)?
            } else {
                anyhow::bail!("Topology file not found: {}", topology_path.display());
            };

            // Find the network
            let network = topology.networks.get_mut(&net)
                .ok_or_else(|| anyhow::anyhow!("Network '{}' not found in topology", net))?;

            // Remove forward from the network
            match network {
                gw_core::Network::Routed(routed) => {
                    let original_len = routed.forwards.len();
                    routed.forwards.retain(|f| f.public != public);

                    if routed.forwards.len() == original_len {
                        anyhow::bail!("Forward for '{}' not found", public);
                    }

                    // Save topology
                    let yaml = serde_yaml::to_string(&topology)?;
                    std::fs::write(topology_path, yaml)?;

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
                Topology::from_file(topology_path)?
            } else {
                anyhow::bail!("Topology file not found: {}", topology_path.display());
            };

            println!("📋 Port Forwards:\n");
            let mut found_any = false;

            for (net_name, network) in &topology.networks {
                if let gw_core::Network::Routed(routed) = network {
                    if !routed.forwards.is_empty() {
                        found_any = true;
                        println!("Network: {}", net_name);
                        for forward in &routed.forwards {
                            println!("  {} -> {}", forward.public, forward.dst);
                        }
                        println!();
                    }
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
        PolicyAction::Set { net, profile } => {
            println!("Setting policy {} for network {}", profile, net);
            println!("(Policy application not yet fully implemented)");
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
            let port: u16 = addr
                .trim_start_matches(':')
                .parse()
                .unwrap_or(9138);

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
