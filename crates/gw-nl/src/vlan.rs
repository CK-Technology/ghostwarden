use anyhow::{Context, Result};
use rtnetlink::{new_connection, Handle};

pub struct VlanManager {
    handle: Handle,
}

impl VlanManager {
    pub async fn new() -> Result<Self> {
        let (connection, handle, _) = new_connection()?;
        tokio::spawn(connection);
        Ok(Self { handle })
    }

    /// Create a VLAN interface
    /// Example: create_vlan("enp6s0", 20, "enp6s0.20")
    pub async fn create_vlan(
        &self,
        parent_iface: &str,
        vlan_id: u16,
        vlan_name: &str,
    ) -> Result<()> {
        println!("Creating VLAN {} on {} (ID: {})", vlan_name, parent_iface, vlan_id);

        // Get parent link index
        let parent_index = self.get_link_by_name(parent_iface).await?;

        // Create VLAN link
        self.handle
            .link()
            .add()
            .vlan(vlan_name.to_string(), parent_index, vlan_id)
            .execute()
            .await
            .context(format!("Failed to create VLAN {}", vlan_name))?;

        println!("Created VLAN interface: {}", vlan_name);

        // Set link up
        let vlan_index = self.get_link_by_name(vlan_name).await?;
        self.handle
            .link()
            .set(vlan_index)
            .up()
            .execute()
            .await
            .context(format!("Failed to bring up VLAN {}", vlan_name))?;

        println!("Set VLAN {} up", vlan_name);
        Ok(())
    }

    /// Delete a VLAN interface
    pub async fn delete_vlan(&self, vlan_name: &str) -> Result<()> {
        let vlan_index = self.get_link_by_name(vlan_name).await?;

        self.handle
            .link()
            .del(vlan_index)
            .execute()
            .await
            .context(format!("Failed to delete VLAN {}", vlan_name))?;

        println!("Deleted VLAN interface: {}", vlan_name);
        Ok(())
    }

    /// Attach VLAN to a bridge
    pub async fn attach_vlan_to_bridge(&self, vlan_name: &str, bridge_name: &str) -> Result<()> {
        let vlan_index = self.get_link_by_name(vlan_name).await?;
        let bridge_index = self.get_link_by_name(bridge_name).await?;

        // Set the VLAN's controller to the bridge
        self.handle
            .link()
            .set(vlan_index)
            .controller(bridge_index)
            .execute()
            .await
            .context(format!("Failed to attach VLAN {} to bridge {}", vlan_name, bridge_name))?;

        println!("Attached VLAN {} to bridge {}", vlan_name, bridge_name);
        Ok(())
    }

    async fn get_link_by_name(&self, name: &str) -> Result<u32> {
        use futures::stream::TryStreamExt;

        let mut links = self.handle.link().get().match_name(name.to_string()).execute();

        if let Some(link) = links.try_next().await? {
            Ok(link.header.index)
        } else {
            anyhow::bail!("Link {} not found", name)
        }
    }
}
