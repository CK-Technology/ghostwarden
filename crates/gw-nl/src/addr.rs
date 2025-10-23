use anyhow::{Context, Result};
use rtnetlink::{new_connection, Handle};
use std::net::IpAddr;

pub struct AddressManager {
    handle: Handle,
}

impl AddressManager {
    pub async fn new() -> Result<Self> {
        let (connection, handle, _) = new_connection()?;
        tokio::spawn(connection);
        Ok(Self { handle })
    }

    pub async fn add_address(&self, iface: &str, cidr: &str) -> Result<()> {
        // Parse CIDR notation (e.g., "10.33.0.1/24")
        let parts: Vec<&str> = cidr.split('/').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid CIDR format: {}", cidr);
        }

        let addr: IpAddr = parts[0]
            .parse()
            .context(format!("Invalid IP address: {}", parts[0]))?;

        let prefix_len: u8 = parts[1]
            .parse()
            .context(format!("Invalid prefix length: {}", parts[1]))?;

        // Get link index
        let link_index = self.get_link_by_name(iface).await?;

        // Add address
        match addr {
            IpAddr::V4(ipv4) => {
                self.handle
                    .address()
                    .add(link_index, ipv4.into(), prefix_len)
                    .execute()
                    .await
                    .context(format!("Failed to add address {} to {}", cidr, iface))?;
            }
            IpAddr::V6(ipv6) => {
                self.handle
                    .address()
                    .add(link_index, ipv6.into(), prefix_len)
                    .execute()
                    .await
                    .context(format!("Failed to add address {} to {}", cidr, iface))?;
            }
        }

        println!("Added address {} to {}", cidr, iface);
        Ok(())
    }

    pub async fn delete_address(&self, iface: &str, cidr: &str) -> Result<()> {
        use futures::stream::TryStreamExt;
        use netlink_packet_route::address::AddressAttribute;

        // Parse CIDR notation
        let parts: Vec<&str> = cidr.split('/').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid CIDR format: {}", cidr);
        }

        let addr: IpAddr = parts[0]
            .parse()
            .context(format!("Invalid IP address: {}", parts[0]))?;

        let prefix_len: u8 = parts[1]
            .parse()
            .context(format!("Invalid prefix length: {}", parts[1]))?;

        // Get link index
        let link_index = self.get_link_by_name(iface).await?;

        // Get existing addresses and find matching ones to delete
        let mut addrs = self.handle.address().get().set_link_index_filter(link_index).execute();

        while let Some(addr_msg) = addrs.try_next().await? {
            // Check if this address matches what we want to delete
            let mut matches = false;

            // Check prefix length matches
            if addr_msg.header.prefix_len != prefix_len {
                continue;
            }

            // Check if the address matches
            for attr in &addr_msg.attributes {
                if let AddressAttribute::Address(msg_addr) = attr {
                    if *msg_addr == addr {
                        matches = true;
                        break;
                    }
                }
            }

            // Delete the matching address
            if matches {
                self.handle
                    .address()
                    .del(addr_msg)
                    .execute()
                    .await
                    .context(format!("Failed to delete address {} from {}", cidr, iface))?;

                println!("Deleted address {} from {}", cidr, iface);
                return Ok(());
            }
        }

        // If we didn't find the address, that's okay (idempotent)
        println!("Address {} not found on {} (already deleted)", cidr, iface);
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
