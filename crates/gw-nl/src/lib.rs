pub mod bridge;
pub mod addr;
pub mod status;
pub mod vlan;

pub use bridge::*;
pub use addr::*;
pub use status::*;
pub use vlan::*;

// Netlink operations for managing links, bridges, VLANs, VXLAN
