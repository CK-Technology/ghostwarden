# GhostWarden v0.3.0 Release Notes

**Release Date**: 2025-01-23  
**Git Tag**: v0.3.0  
**Commit**: 8099c89

---

## 🎉 Overview

GhostWarden v0.3.0 represents a major milestone, transforming the project from early development into a production-ready Linux network orchestration platform with professional-grade packaging, comprehensive documentation, and complete distribution infrastructure.

---

## ✨ What's New

### Production Hardening
- ✅ **Complete rollback implementation** - Full resource cleanup (bridges, addresses, nftables, dnsmasq)
- ✅ **Topology validation layer** - CIDR overlaps, port ranges, IP validation, gateway checks
- ✅ **Port forward management** - CLI commands for add/remove/list port forwards
- ✅ **Address deletion** - Proper netlink-based address removal with IPv4/IPv6 support
- ✅ **Prometheus metrics** - Full HTTP exporter on port 9138 with bridge/nft/DHCP metrics

### Complete Packaging Infrastructure
- 📦 **AUR Package** - Ready for Arch Linux distribution (`yay -S ghostwarden`)
- 🔧 **Systemd Units** - Security-hardened services (gwarden.service, gwarden-metrics.service)
- ⚙️ **Daemon Configuration** - Comprehensive TOML config with all settings
- 📚 **8 Man Pages** - Complete documentation (gwarden, net, vm, forward, policy, metrics, doctor, graph)
- 🐚 **Shell Completions** - Bash, Zsh, and advanced Gshell completions
- 🚀 **Installation Script** - Automated installer with uninstall support

### Workspace Restructure
Reorganized into proper Rust workspace with 9 crates:
- `gw-cli` - Main binary
- `gw-core` - Core logic (topology, validation, rollback)
- `gw-nl` - Netlink operations
- `gw-nft` - nftables management
- `gw-dhcpdns` - dnsmasq integration
- `gw-libvirt` - VM management
- `gw-tui` - Terminal UI
- `gw-metrics` - Prometheus exporter
- `gw-troubleshoot` - Diagnostics

---

## 📊 Statistics

- **79 files changed**
- **13,408 insertions** (+), **11 deletions** (-)
- **~7,000 lines** of code
- **1,845 lines** in release/ directory
- **8 tests** (validator, metrics, docker)
- **8 man pages**
- **3 shell completion systems**

---

## 🚀 Installation

### Arch Linux (AUR)
```bash
yay -S ghostwarden
# or
paru -S ghostwarden
```

### Manual Installation
```bash
git clone https://github.com/CK-Technology/ghostwarden.git
cd ghostwarden
sudo ./release/install.sh --build
```

### From Source
```bash
cargo build --release
sudo install -Dm755 target/release/gwarden /usr/bin/gwarden
```

---

## 📖 Quick Start

1. **Configure topology**:
   ```bash
   sudo nano /etc/ghostwarden/ghostnet.yaml
   ```

2. **Preview changes**:
   ```bash
   gwarden net plan
   ```

3. **Apply with rollback safety**:
   ```bash
   sudo gwarden net apply --commit --confirm 30s
   ```

4. **Enable metrics** (optional):
   ```bash
   sudo systemctl enable --now gwarden-metrics.service
   curl http://localhost:9138/metrics
   ```

5. **Run diagnostics**:
   ```bash
   sudo gwarden doctor
   ```

6. **Launch TUI**:
   ```bash
   gwarden tui
   ```

---

## 🔧 Configuration

### Main Topology
**Location**: `/etc/ghostwarden/ghostnet.yaml`

Example:
```yaml
version: 1
networks:
  nat_dev:
    type: routed
    cidr: 10.33.0.0/24
    dhcp: true
    dns: true
    masq_out: enp6s0
    forwards:
      - { public: ":8080/tcp", dst: "10.33.0.10:80" }
```

### Daemon Config
**Location**: `/etc/ghostwarden/daemon.toml`

Controls metrics server, rollback behavior, logging, integrations.

### Policy Profiles
**Location**: `/etc/ghostwarden/policies/*.yaml`

Pre-configured profiles: `routed-tight`, `public-web`, `l2-lan`

---

## 🛡️ Security Features

### Systemd Hardening
- Minimal capabilities (CAP_NET_ADMIN, CAP_NET_BIND_SERVICE)
- ProtectSystem=strict, ProtectHome=true
- NoNewPrivileges, MemoryDenyWriteExecute
- Restricted namespaces and address families
- Dynamic users for metrics service

### Validation
- CIDR overlap detection
- Port range validation (1-65535)
- IP address format validation
- Gateway-in-CIDR validation
- Protocol validation (tcp/udp/sctp)

### Rollback Safety
- Automatic rollback on timeout
- SSH connectivity monitoring
- Full resource cleanup
- Rollback history tracking

---

## 📚 Documentation

### Man Pages
```bash
man gwarden              # Main manual
man gwarden-net          # Network management
man gwarden-vm           # VM management
man gwarden-forward      # Port forwarding
man gwarden-policy       # Policy management
man gwarden-metrics      # Metrics server
man gwarden-doctor       # Diagnostics
man gwarden-graph        # Visualization
```

### Shell Completions

#### Bash
Auto-loaded from `/usr/share/bash-completion/completions/`

#### Zsh
Auto-loaded from `/usr/share/zsh/site-functions/`

#### Gshell (Advanced)
Features keyboard shortcuts:
- `Ctrl-G-P` → gwarden net plan
- `Ctrl-G-A` → gwarden net apply --commit --confirm 30s
- `Ctrl-G-S` → gwarden net status
- `Ctrl-G-D` → gwarden doctor
- `Ctrl-G-T` → gwarden tui

Plus dynamic completion, validation, hints, and aliases.

---

## 🔄 Migration from v0.1.0

### Breaking Changes
1. Config path changed: `/etc/ghostnet/` → `/etc/ghostwarden/`
2. Workspace structure (single binary → 9 crates)
3. Systemd unit naming standardized

### Migration Steps
```bash
# Move config directory
sudo mv /etc/ghostnet /etc/ghostwarden

# Reload systemd
sudo systemctl daemon-reload

# Reinstall
yay -S ghostwarden
```

---

## 🐛 Known Issues

- Test coverage at 8 tests (needs expansion)
- Some metrics fields unused (dead code warnings)
- VXLAN support stubbed (planned for v0.5.0)

---

## 🎯 Roadmap

### v0.4.0 - Proxmox Integration (Q2 2025)
- Proxmox API client
- VM import/export
- Cluster support
- VXLAN overlays

### v0.5.0 - Advanced Networking (Q3 2025)
- BGP integration
- QoS/traffic control
- VPN support (WireGuard)
- Time-based rules

### v0.6.0 - Observability (Q4 2025)
- eBPF flow monitoring
- Live TUI flow view
- Enhanced Prometheus metrics
- Grafana dashboards

---

## 🙏 Acknowledgments

Built with:
- Rust 2024 edition
- nftables, dnsmasq
- rtnetlink, axum, prometheus
- ratatui, clap

Special thanks to the Rust community and all contributors.

---

## 📞 Support

- **GitHub**: https://github.com/CK-Technology/ghostwarden
- **Issues**: https://github.com/CK-Technology/ghostwarden/issues
- **Email**: ckelley@ghostkellz.sh
- **AUR**: https://aur.archlinux.org/packages/ghostwarden

---

## 📜 License

MIT © 2025 CK Technology / GhostKellz

---

**Full Changelog**: https://github.com/CK-Technology/ghostwarden/compare/v0.1.0...v0.3.0
