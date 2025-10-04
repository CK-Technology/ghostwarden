🛡️ Ghostwarden
<div align="center">
<img src="assets/icons/ghostwarden.png" alt="Ghostwarden Icon" width="175" height="175">

**Linux Network Guardian — nftables · Bridges · Firewalls · Visibility**

*Like ufw++ but for Linux bridges, nftables, iptables, and policies*

![rust](https://img.shields.io/badge/Built%20with-Rust-orange?logo=rust)
![nftables](https://img.shields.io/badge/Firewall-nftables-blue?logo=linux)
![bridge](https://img.shields.io/badge/Bridging-L2%20%26%20L3-green?logo=ethernet)
![policy](https://img.shields.io/badge/Policy-Zero%20Trust-red)
![crowdsec](https://img.shields.io/badge/Integration-CrowdSec-4B7BBE?logo=crowdsource)
![wazuh](https://img.shields.io/badge/Integration-Wazuh-005B94)
![prometheus](https://img.shields.io/badge/Metrics-Prometheus-DA4E2B?logo=prometheus)
![proxmox](https://img.shields.io/badge/Compatible-Proxmox%20VE-orange?logo=proxmox)
![archlinux](https://img.shields.io/badge/Tested%20on-Arch%20Linux-1793D1?logo=archlinux)
![docker](https://img.shields.io/badge/Optional-Docker%20Ready-blue?logo=docker)
![license](https://img.shields.io/badge/License-MIT-lightgrey)

</div>
## Overview

Ghostwarden is a Rust-powered network security orchestrator that unifies nftables, Linux bridges, and container/VM networks under a single declarative and human-friendly CLI.

It helps you see, manage, and enforce what your containers, VMs, and hosts can talk to — across NAT, VLANs, VXLANs, and SDN bridges — with rollback-safe, zero-trust policies.

Ideal for Arch Linux, Proxmox, and developer labs, Ghostwarden acts as your network's guardian angel: translating YAML/CLI configs into live, auditable firewall and routing state.

## ✨ Features

### Unified Networking UX

- Define bridges, NATs, and VLANs in a simple YAML topology
- Apply in one command (`gwarden apply --commit`)
- Rollback automatically on disconnects or errors

### nftables Policy Engine

- Layer-3 firewall & NAT management via JSON rulesets
- Dynamic port-forwards & per-network profiles
- CrowdSec/Wazuh hooks for ban decisions

### Observability & Visualization

- Built-in Prometheus `/metrics` exporter
- `gwarden graph --mermaid` for live topology diagrams
- See which containers/VMs are talking and how

### Cluster-Aware (Future)

- Integrates with Proxmox SDN bridges and libvirt networks
- Detects `vmbr*`, `tap*`, `veth*`, and `vxlan*` interfaces automatically

### Safe by Default

- Transactional apply/rollback
- Zero-trust profiles (e.g. `routed-tight`, `public-web`)
- Whitelists, TTLs, and conflict detection

## 📦 Components

| Component | Description |
|-----------|-------------|
| `gward` | Core daemon — applies nftables & manages state |
| `net-core` | Topology model, diff planner, rollback engine |
| `net-nft` | nftables JSON builder/verifier | 
| `net-dhcpdns` | dnsmasq/CoreDNS management |
| `net-bridge` | Handles Linux bridge/VLAN/VXLAN creation |
| `integrations/` | CrowdSec, Wazuh, Prometheus exporters |
| `net-cli` | `gwarden` commands (powered by clap) |
## 🚀 Quick Start

```bash
# Build
cargo build --release

# Preview your current network plan
gwarden plan

# Apply network definitions (with rollback safety)
gwarden apply --commit --confirm 30s
```

### Example Topology (`/etc/ghostnet/workstation.yml`)

```yaml
version: 1
interfaces:
  uplink: enp6s0

networks:
  br_work:
    type: bridge
    iface: br-work
    vlan: 20
    members:
      - vm: devbox-01
      - vm: devbox-02

  nat_dev:
    type: routed
    cidr: 10.33.0.0/24
    dhcp: true
    dns: true
    masq_out: enp6s0
    forwards:
      - { public: ":4022/tcp", dst: "10.33.0.10:22" }
```

## 🧠 Example CLI

```bash
# Create and apply a NAT network
gwarden net create nat/dev --cidr 10.33.0.0/24 --dhcp --dns --masq via enp6s0

# Add a port forward
gwarden forward add nat/dev --dst 10.33.0.10:22 --public :4022/tcp

# Generate a live diagram
gwarden graph --mermaid

# Monitor metrics
curl localhost:9138/metrics
```

## 🗺 Roadmap

- [ ] MVP: nftables + dnsmasq orchestration
- [ ] Prometheus metrics + live graph
- [ ] Proxmox/libvirt integration
- [ ] CrowdSec + Wazuh ban sync
- [ ] VXLAN peer management
- [ ] eBPF traffic sampling dashboard

## 🧱 Stack

- 🦀 Rust 2024 edition
- 🧩 `neli`, `serde_yaml`, `nftables-json`, `clap`, `axum`, `prometheus`
- ⚙️ Optional: systemd integration, journald logging
- 🧠 Future: Ratatui TUI dashboard for live network map

## 📜 License

MIT © 2025 CK Technology / GhostKellz
