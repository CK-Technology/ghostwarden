pub mod addr;
pub mod bridge;
pub mod status;
pub mod vlan;

pub use addr::*;
pub use bridge::*;
pub use status::*;
pub use vlan::*;

// Netlink operations for managing links, bridges, VLANs, VXLAN
