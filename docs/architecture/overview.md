# Architecture Overview

Ghostwarden is designed around a simple pipeline:

```mermaid
flowchart TD
    TOPO["Topology TOML/YAML\n/etc/gwarden/ghostnet.toml"] --> CORE

    subgraph CORE["gw-core"]
        PARSE["parser\nTopology::from_file"] --> VALID["validator\nCIDR / iface / profile checks"]
        VALID --> PLAN["planner\nordered Plan actions"]
    end

    PLAN --> NL["gw-nl\nbridge / address / VLAN"]
    PLAN --> NFT["gw-nft\nnftables rulesets (NAT/forward/filter)"]
    PLAN --> DNS["gw-dhcpdns\ndnsmasq configs + leases"]
    PLAN --> VIRT["gw-libvirt\nVM tap attachment"]

    NL --> OBS["status · diagnostics · metrics · rollback"]
    NFT --> OBS
    DNS --> OBS
    VIRT --> OBS
```

## Design Goals

- CLI-first operations that are easy to audit over SSH.
- Declarative topology files that can be versioned and reviewed.
- nftables-native firewall and NAT behavior.
- Rollback-aware host changes.
- Practical coexistence with Proxmox, libvirt, Docker, and NetworkManager.

## Current Boundaries

Ghostwarden does not try to replace every host network manager. It should own the networks declared in its topology files and avoid fighting tools that own unrelated interfaces.

The project currently favors explicit system commands and host-native tools over a daemon-only architecture. A daemon mode can build on the same crates after state persistence and apply transactions are hardened.
