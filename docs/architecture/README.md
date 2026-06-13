# Architecture

Ghostwarden is a Rust workspace split by responsibility. The CLI wires together topology parsing, planning, nftables generation, netlink operations, dnsmasq helpers, libvirt helpers, metrics, troubleshooting, and the TUI.

## Pages

- [Overview](overview.md)
- [Diagrams](diagrams.md)
- [Planner](planner.md)
- [Rollback](rollback.md)

## Workspace Crates

| Crate | Responsibility |
|-------|----------------|
| `gw-cli` | `gwarden` command-line interface |
| `gw-core` | topology, planning, policy, rollback, status models |
| `gw-nft` | nftables rulesets, diffs, status collection |
| `gw-nl` | netlink bridge, address, VLAN, and status helpers |
| `gw-dhcpdns` | dnsmasq config and lease reading |
| `gw-libvirt` | VM bridge attachment helpers |
| `gw-metrics` | Prometheus metrics server |
| `gw-troubleshoot` | doctor diagnostics |
| `gw-tui` | ratatui dashboard |
