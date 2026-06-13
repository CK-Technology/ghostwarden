//! Live nftables integration tests (apply / snapshot / restore).
//!
//! These run real `nft` commands and therefore require root. They are `#[ignore]`
//! by default; run them in a throwaway network namespace:
//!
//! ```bash
//! sudo -E unshare --net cargo test -p gw-nft --test netns_nft -- --ignored --test-threads=1
//! ```
//!
//! The suite uses a `gwt-` table name so it never collides with real rulesets,
//! and deletes the table on the way out.

use gw_nft::NftManager;

fn require_root() -> bool {
    let is_root = std::process::Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim() == "0")
        .unwrap_or(false);

    if !is_root {
        eprintln!("skipping: requires root + nftables (see module docs)");
    }
    is_root
}

#[tokio::test]
#[ignore = "requires root + nftables"]
async fn apply_snapshot_and_restore_roundtrip() {
    if !require_root() {
        return;
    }
    let mgr = NftManager::new();
    let table = "gwt-test";

    // Start clean.
    let _ = mgr.delete_table(table).await;
    assert!(
        mgr.snapshot_table(table).await.unwrap().is_none(),
        "table should not exist yet"
    );

    let ruleset = mgr
        .create_nat_ruleset(table, "gwt-br9", "10.124.0.0/24", "10.124.0.1", "lo", &[])
        .expect("generate ruleset");

    // First apply: no prior snapshot exists.
    let prior = mgr.apply_ruleset(table, &ruleset).await.expect("apply");
    assert!(prior.is_none(), "no snapshot expected on first apply");
    assert!(
        mgr.list_tables().await.unwrap().iter().any(|t| t == table),
        "table should be live after apply"
    );

    // Second apply: the manager must hand back a snapshot of the live table.
    let snapshot = mgr.apply_ruleset(table, &ruleset).await.expect("re-apply");
    assert!(snapshot.is_some(), "snapshot expected on second apply");

    // Restore with no snapshot deletes the table (the rollback "fresh table" path).
    mgr.restore_table_from_snapshot(table, None)
        .await
        .expect("restore/delete");
    assert!(
        !mgr.list_tables().await.unwrap().iter().any(|t| t == table),
        "table should be gone after restoring an empty snapshot"
    );
}
