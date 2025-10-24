use anyhow::Result;
use gw_core::BridgeStatus;
use rtnetlink::Handle;

pub struct StatusCollector {
    handle: Handle,
}

impl StatusCollector {
    pub async fn new() -> Result<Self> {
        use rtnetlink::new_connection;
        let (connection, handle, _) = new_connection()?;
        tokio::spawn(connection);
        Ok(Self { handle })
    }

    pub async fn collect_bridge_status(&self) -> Result<Vec<BridgeStatus>> {
        use futures::stream::TryStreamExt;
        use netlink_packet_route::link::LinkAttribute;

        let mut bridges = vec![];
        let mut links = self.handle.link().get().execute();

        while let Some(link) = links.try_next().await? {
            let name = link.attributes.iter().find_map(|attr| {
                if let LinkAttribute::IfName(n) = attr {
                    Some(n.clone())
                } else {
                    None
                }
            });

            if let Some(name) = name {
                // Check if it's a bridge
                let is_bridge = link
                    .attributes
                    .iter()
                    .any(|attr| matches!(attr, LinkAttribute::LinkInfo(_)));

                if is_bridge || name.starts_with("br-") {
                    // Check if link is up by looking at flags
                    let state = if link
                        .header
                        .flags
                        .contains(&netlink_packet_route::link::LinkFlag::Up)
                    {
                        "UP"
                    } else {
                        "DOWN"
                    }
                    .to_string();

                    // Get addresses for this bridge
                    let addresses = self.get_addresses_for_link(link.header.index).await?;

                    bridges.push(BridgeStatus {
                        name,
                        state,
                        addresses,
                        members: vec![], // TODO: get bridge members
                    });
                }
            }
        }

        Ok(bridges)
    }

    async fn get_addresses_for_link(&self, link_index: u32) -> Result<Vec<String>> {
        use futures::stream::TryStreamExt;
        use netlink_packet_route::address::AddressAttribute;

        let mut addresses = vec![];
        let mut addrs = self
            .handle
            .address()
            .get()
            .set_link_index_filter(link_index)
            .execute();

        while let Some(addr) = addrs.try_next().await? {
            for attr in &addr.attributes {
                if let AddressAttribute::Address(ip) = attr {
                    let addr_str =
                        format!("{}/{}", std::net::IpAddr::from(*ip), addr.header.prefix_len);
                    addresses.push(addr_str);
                }
            }
        }

        Ok(addresses)
    }
}
