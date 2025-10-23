use anyhow::{Context, Result};
use rtnetlink::{new_connection, Handle};
use netlink_packet_route::link::{LinkAttribute, LinkFlag, LinkMessage};
use futures::stream::TryStreamExt;

pub struct BridgeManager {
    handle: Handle,
}

impl BridgeManager {
    pub async fn new() -> Result<Self> {
        let (connection, handle, _) = new_connection()?;
        tokio::spawn(connection);
        Ok(Self { handle })
    }

    pub async fn create_bridge(&self, name: &str) -> Result<()> {
        // Check if bridge already exists
        if self.bridge_exists(name).await? {
            println!("Bridge {} already exists, skipping creation", name);
            return Ok(());
        }

        // Create bridge
        self.handle
            .link()
            .add()
            .bridge(name.to_string())
            .execute()
            .await
            .context(format!("Failed to create bridge {}", name))?;

        println!("Created bridge: {}", name);

        // Set bridge up
        let link = self.get_link_by_name(name).await?;
        self.handle
            .link()
            .set(link)
            .up()
            .execute()
            .await
            .context(format!("Failed to set bridge {} up", name))?;

        println!("Set bridge {} up", name);
        Ok(())
    }

    pub async fn delete_bridge(&self, name: &str) -> Result<()> {
        let link = self.get_link_by_name(name).await?;

        // Set link down first
        self.handle
            .link()
            .set(link)
            .down()
            .execute()
            .await
            .context(format!("Failed to set bridge {} down", name))?;

        // Delete the bridge
        self.handle
            .link()
            .del(link)
            .execute()
            .await
            .context(format!("Failed to delete bridge {}", name))?;

        println!("Deleted bridge: {}", name);
        Ok(())
    }

    pub async fn list_bridges(&self) -> Result<Vec<String>> {
        use netlink_packet_route::link::LinkAttribute;

        let mut links = self.handle.link().get().execute();
        let mut bridges = Vec::new();

        while let Some(link) = links.try_next().await? {
            let name = link.attributes.iter().find_map(|attr| {
                if let LinkAttribute::IfName(n) = attr {
                    Some(n.clone())
                } else {
                    None
                }
            });

            if let Some(name) = name {
                // Check if it's a bridge by looking for bridge info
                let is_bridge = link.attributes.iter().any(|attr| {
                    matches!(attr, LinkAttribute::LinkInfo(_))
                });
                if is_bridge {
                    bridges.push(name);
                }
            }
        }

        Ok(bridges)
    }

    pub async fn bridge_exists(&self, name: &str) -> Result<bool> {
        match self.get_link_by_name(name).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    async fn get_link_by_name(&self, name: &str) -> Result<u32> {
        let link_msg = self.get_link_message_by_name(name).await?;
        Ok(link_msg.header.index)
    }

    async fn get_link_message_by_name(&self, name: &str) -> Result<LinkMessage> {
        let mut links = self.handle.link().get().match_name(name.to_string()).execute();

        if let Some(link) = links.try_next().await? {
            Ok(link)
        } else {
            anyhow::bail!("Link {} not found", name)
        }
    }

    /// Get detailed bridge information
    pub async fn get_bridge_info(&self, bridge_name: &str) -> Result<BridgeInfo> {
        let link = self.get_link_message_by_name(bridge_name).await?;

        // Get bridge state
        let is_up = link.header.flags.contains(&LinkFlag::Up);

        // Get bridge members (ports)
        let members = self.get_bridge_members(bridge_name).await?;

        // Get MTU
        let mut mtu = 1500u32; // Default
        for attr in &link.attributes {
            if let LinkAttribute::Mtu(m) = attr {
                mtu = *m;
            }
        }

        Ok(BridgeInfo {
            name: bridge_name.to_string(),
            index: link.header.index,
            is_up,
            mtu,
            members,
        })
    }

    /// Get interfaces attached to a bridge
    pub async fn get_bridge_members(&self, bridge_name: &str) -> Result<Vec<String>> {
        let bridge = self.get_link_message_by_name(bridge_name).await?;
        let bridge_index = bridge.header.index;

        let mut links = self.handle.link().get().execute();
        let mut members = Vec::new();

        while let Some(link) = links.try_next().await? {
            // Check if this link is attached to our bridge (has this bridge as controller)
            for attr in &link.attributes {
                if let LinkAttribute::Controller(controller_index) = attr {
                    if *controller_index == bridge_index {
                        // This link is attached to our bridge
                        if let Some(name) = link.attributes.iter().find_map(|a| {
                            if let LinkAttribute::IfName(n) = a {
                                Some(n.clone())
                            } else {
                                None
                            }
                        }) {
                            members.push(name);
                        }
                    }
                }
            }
        }

        Ok(members)
    }

    /// Set bridge MTU
    pub async fn set_mtu(&self, bridge_name: &str, mtu: u32) -> Result<()> {
        let link_index = self.get_link_by_name(bridge_name).await?;

        self.handle
            .link()
            .set(link_index)
            .mtu(mtu)
            .execute()
            .await
            .context(format!("Failed to set MTU for bridge {}", bridge_name))?;

        println!("Set MTU for bridge {} to {}", bridge_name, mtu);
        Ok(())
    }

    /// Attach an interface to a bridge
    pub async fn attach_interface_to_bridge(
        &self,
        interface: &str,
        bridge: &str,
    ) -> Result<()> {
        let iface_index = self.get_link_by_name(interface).await?;
        let bridge_index = self.get_link_by_name(bridge).await?;

        self.handle
            .link()
            .set(iface_index)
            .controller(bridge_index)
            .execute()
            .await
            .context(format!(
                "Failed to attach {} to bridge {}",
                interface, bridge
            ))?;

        println!("Attached interface {} to bridge {}", interface, bridge);
        Ok(())
    }

    /// Detach an interface from its bridge
    pub async fn detach_interface(&self, interface: &str) -> Result<()> {
        let link_index = self.get_link_by_name(interface).await?;

        // Setting controller to 0 detaches from bridge
        self.handle
            .link()
            .set(link_index)
            .controller(0)
            .execute()
            .await
            .context(format!("Failed to detach interface {}", interface))?;

        println!("Detached interface {} from bridge", interface);
        Ok(())
    }

    pub async fn enable_forwarding(&self, name: &str) -> Result<()> {
        // Enable IPv4 forwarding via sysctl
        let sysctl_path = format!("/proc/sys/net/ipv4/conf/{}/forwarding", name);
        std::fs::write(&sysctl_path, "1")
            .context(format!("Failed to enable forwarding on {}", name))?;

        println!("Enabled forwarding on {}", name);
        Ok(())
    }
}

/// Bridge information structure
#[derive(Debug, Clone)]
pub struct BridgeInfo {
    pub name: String,
    pub index: u32,
    pub is_up: bool,
    pub mtu: u32,
    pub members: Vec<String>,
}
