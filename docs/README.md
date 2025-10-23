# Ghostwarden Documentation

Pragmatic network orchestration for Arch Linux with nftables, libvirt, and Linux virtual networking.

## Quick Links

- [Getting Started](guides/quickstart.md)
- [Installation on Arch Linux](guides/arch-installation.md)
- [Architecture Overview](architecture/overview.md)
- [CLI Reference](reference/cli.md)
- [Policy Profiles](reference/policy-profiles.md)
- [Proxmox VE Integration](guides/proxmox-integration.md)

## What is Ghostwarden?

Ghostwarden is a network orchestration tool that helps you manage:
- **Virtual networks** (bridges, VLANs, VXLAN)
- **nftables firewall rules** with policy profiles
- **DHCP/DNS** via dnsmasq
- **Libvirt VM networking**
- **NAT/routing** with port forwarding

All with a crisp CLI, live TUI, and transactional apply with rollback.

## Documentation Structure

```
docs/
├─ guides/
│  ├─ quickstart.md                    # 5-minute setup
│  ├─ arch-installation.md             # Arch Linux deployment
│  ├─ proxmox-integration.md           # Proxmox VE setup
│  ├─ migration-from-ufw.md            # Migrating from UFW
│  └─ best-practices.md                # Production tips
├─ reference/
│  ├─ cli.md                           # All commands
│  ├─ topology-yaml.md                 # ghostnet.yaml format
│  ├─ policy-profiles.md               # Policy system
│  └─ nftables-integration.md          # nftables details
├─ architecture/
│  ├─ overview.md                      # High-level design
│  ├─ planner.md                       # Plan generation
│  ├─ executor.md                      # Execution engine
│  └─ rollback.md                      # Rollback mechanism
└─ examples/
   ├─ nat-dev-network.md               # Basic NAT setup
   ├─ vlan-segmentation.md             # VLAN example
   ├─ proxmox-cluster.md               # Proxmox example
   └─ multi-tenant.md                  # Multi-tenant setup
```

## Current Status

**Version:** 0.1.0
**Edition:** Rust 2024
**License:** MIT
**Author:** Christopher Kelley <ckelley@ghostkellz.sh>

### Implemented Features

- ✅ Network topology IR (YAML-based)
- ✅ Bridge/VLAN/VXLAN management (rtnetlink)
- ✅ nftables ruleset generation (MASQUERADE, DNAT/SNAT)
- ✅ DHCP/DNS via dnsmasq
- ✅ Conflict detection (NetworkManager, Docker, UFW, firewalld)
- ✅ Rollback with timeout
- ✅ Libvirt VM attach/detach
- ✅ Policy profiles (routed-tight, public-web, l2-lan)
- ✅ Terminal UI (ratatui)
- ✅ Network status reporting

### Roadmap

See [ROADMAP.md](../ROADMAP.md) for:
- Production hardening checklist
- Proxmox VE integration plans
- eBPF flow monitoring
- REST API + web GUI
- Systemd daemon mode
- Package formats (Arch PKGBUILD, .deb)

## Support

- **Issues:** https://github.com/ghostkellz/ghostwarden/issues
- **Discussions:** https://github.com/ghostkellz/ghostwarden/discussions
