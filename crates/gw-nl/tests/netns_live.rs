//! Live netlink integration tests (bridges, addresses, VLANs).
//!
//! These mutate real host networking, so they require root (`CAP_NET_ADMIN`).
//! They are `#[ignore]` by default; run them inside a throwaway network
//! namespace so they cannot disturb production interfaces:
//!
//! ```bash
//! sudo -E unshare --net cargo test -p gw-nl --test netns_live -- --ignored --test-threads=1
//! ```
//!
//! Each test cleans up after itself and uses a `gwt-` name prefix to stay clear
//! of real interfaces.

use std::process::Command;

use gw_nl::{AddressManager, BridgeManager, VlanManager};

/// Skip (rather than fail) when not run as root, so `--ignored` runs are still
/// meaningful on unprivileged machines.
fn require_root() -> bool {
    let is_root = Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim() == "0")
        .unwrap_or(false);

    if !is_root {
        eprintln!("skipping: requires root + network namespace (see module docs)");
    }
    is_root
}

fn link_exists(name: &str) -> bool {
    Command::new("ip")
        .args(["link", "show", name])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn iface_has_addr(iface: &str, addr: &str) -> bool {
    Command::new("ip")
        .args(["addr", "show", "dev", iface])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|out| out.contains(addr))
        .unwrap_or(false)
}

#[tokio::test]
#[ignore = "requires root + network namespace"]
async fn bridge_create_and_delete() {
    if !require_root() {
        return;
    }
    let mgr = BridgeManager::new().await.expect("bridge manager");
    let name = "gwt-br0";

    if mgr.bridge_exists(name).await.unwrap_or(false) {
        let _ = mgr.delete_bridge(name).await;
    }

    mgr.create_bridge(name).await.expect("create bridge");
    assert!(
        mgr.bridge_exists(name).await.unwrap(),
        "bridge should exist"
    );
    assert!(link_exists(name), "kernel should report the link");

    mgr.delete_bridge(name).await.expect("delete bridge");
    assert!(
        !mgr.bridge_exists(name).await.unwrap(),
        "bridge should be gone"
    );
}

#[tokio::test]
#[ignore = "requires root + network namespace"]
async fn address_add_and_remove() {
    if !require_root() {
        return;
    }
    let bridge_mgr = BridgeManager::new().await.expect("bridge manager");
    let addr_mgr = AddressManager::new().await.expect("address manager");
    let iface = "gwt-br1";
    let cidr = "10.123.0.1/24";

    let _ = bridge_mgr.delete_bridge(iface).await;
    bridge_mgr
        .create_bridge(iface)
        .await
        .expect("create bridge");

    addr_mgr
        .add_address(iface, cidr)
        .await
        .expect("add address");
    assert!(iface_has_addr(iface, "10.123.0.1/24"), "address present");

    addr_mgr
        .delete_address(iface, cidr)
        .await
        .expect("remove address");
    assert!(!iface_has_addr(iface, "10.123.0.1/24"), "address removed");

    bridge_mgr
        .delete_bridge(iface)
        .await
        .expect("cleanup bridge");
}

#[tokio::test]
#[ignore = "requires root + network namespace"]
async fn vlan_create_and_delete() {
    if !require_root() {
        return;
    }
    let bridge_mgr = BridgeManager::new().await.expect("bridge manager");
    let vlan_mgr = VlanManager::new().await.expect("vlan manager");
    let parent = "gwt-par0";
    let vlan = "gwt-par0.42";

    let _ = vlan_mgr.delete_vlan(vlan).await;
    let _ = bridge_mgr.delete_bridge(parent).await;

    bridge_mgr
        .create_bridge(parent)
        .await
        .expect("create parent");
    vlan_mgr
        .create_vlan(parent, 42, vlan)
        .await
        .expect("create vlan");
    assert!(link_exists(vlan), "vlan link should exist");

    // The fix this suite guards: VLAN teardown must use the VLAN delete path,
    // not bridge deletion.
    vlan_mgr.delete_vlan(vlan).await.expect("delete vlan");
    assert!(!link_exists(vlan), "vlan link should be gone");

    bridge_mgr
        .delete_bridge(parent)
        .await
        .expect("cleanup parent");
}
