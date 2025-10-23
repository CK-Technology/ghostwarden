use crate::topology::Topology;
use std::fmt;

#[derive(Debug, Clone)]
pub struct Plan {
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone)]
pub enum Action {
    CreateBridge { name: String, cidr: Option<String> },
    AddAddress { iface: String, addr: String },
    EnableForwarding { iface: String },
    CreateNftRuleset { table: String, rules: String },
    StartDnsmasq { config_path: String },
    CreateVlan { parent: String, vlan_id: u16, name: String },
    AttachVlanToBridge { vlan: String, bridge: String },
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Action::CreateBridge { name, cidr } => {
                write!(f, "Create bridge: {}", name)?;
                if let Some(cidr) = cidr {
                    write!(f, " ({})", cidr)?;
                }
                Ok(())
            }
            Action::AddAddress { iface, addr } => {
                write!(f, "Add address {} to {}", addr, iface)
            }
            Action::EnableForwarding { iface } => {
                write!(f, "Enable forwarding on {}", iface)
            }
            Action::CreateNftRuleset { table, .. } => {
                write!(f, "Create nftables table: {}", table)
            }
            Action::StartDnsmasq { config_path } => {
                write!(f, "Start dnsmasq with config: {}", config_path)
            }
            Action::CreateVlan { parent, vlan_id, name } => {
                write!(f, "Create VLAN {} on {} (ID: {})", name, parent, vlan_id)
            }
            Action::AttachVlanToBridge { vlan, bridge } => {
                write!(f, "Attach VLAN {} to bridge {}", vlan, bridge)
            }
        }
    }
}

impl Plan {
    pub fn new() -> Self {
        Self {
            actions: Vec::new(),
        }
    }

    pub fn from_topology(topology: &Topology) -> anyhow::Result<Self> {
        let mut plan = Plan::new();

        for (net_name, network) in &topology.networks {
            match network {
                crate::topology::Network::Routed(routed) => {
                    plan.actions.push(Action::CreateBridge {
                        name: format!("br-{}", net_name),
                        cidr: Some(routed.cidr.clone()),
                    });
                    plan.actions.push(Action::AddAddress {
                        iface: format!("br-{}", net_name),
                        addr: routed.cidr.clone(),
                    });
                    plan.actions.push(Action::EnableForwarding {
                        iface: format!("br-{}", net_name),
                    });

                    if routed.dhcp {
                        plan.actions.push(Action::StartDnsmasq {
                            config_path: format!("/etc/dnsmasq.d/gw-{}.conf", net_name),
                        });
                    }
                }
                crate::topology::Network::Bridge(bridge) => {
                    // Create VLAN if specified
                    if let Some(vlan_id) = bridge.vlan {
                        if let Some(uplink) = topology.interfaces.get("uplink") {
                            let vlan_name = format!("{}.{}", uplink, vlan_id);
                            plan.actions.push(Action::CreateVlan {
                                parent: uplink.clone(),
                                vlan_id,
                                name: vlan_name.clone(),
                            });

                            // Attach VLAN to bridge
                            plan.actions.push(Action::AttachVlanToBridge {
                                vlan: vlan_name,
                                bridge: bridge.iface.clone(),
                            });
                        }
                    }

                    plan.actions.push(Action::CreateBridge {
                        name: bridge.iface.clone(),
                        cidr: None,
                    });
                }
                crate::topology::Network::Vxlan(_vxlan) => {
                    // TODO: VXLAN support
                }
            }
        }

        Ok(plan)
    }

    pub fn display(&self) {
        println!("Plan ({} actions):", self.actions.len());
        for (i, action) in self.actions.iter().enumerate() {
            println!("  {}. {}", i + 1, action);
        }
    }
}

impl Default for Plan {
    fn default() -> Self {
        Self::new()
    }
}
