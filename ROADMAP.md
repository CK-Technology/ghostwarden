# Ghostwarden Roadmap

## Phase 1: Production Hardening (v0.2.0) 🚧

**Goal:** Make Ghostwarden production-ready for Arch Linux deployments.

### Critical Path

- [ ] **Full nftables Integration**
  - [ ] Complete MASQUERADE rule generation
  - [ ] DNAT/SNAT for port forwards (basic impl done)
  - [ ] Policy profile → nftables rule compiler
  - [ ] Atomic ruleset replacement
  - [ ] Rule validation before apply

- [ ] **Rollback Improvements**
  - [ ] Snapshot state before apply
  - [ ] Actual SSH connectivity check
  - [ ] Bridge/addr/nft cleanup in rollback
  - [ ] Rollback history tracking
  - [ ] Manual rollback command: `gwarden net rollback`

- [ ] **Safety & Validation**
  - [ ] YAML schema validation
  - [ ] CIDR/IP validation
  - [ ] Port range validation
  - [ ] Detect CIDR overlaps
  - [ ] Warn on dangerous operations (flush all rules, etc.)

- [ ] **Testing**
  - [ ] Netns integration tests
  - [ ] Bridge creation tests
  - [ ] nftables rule tests
  - [ ] Rollback simulation tests
  - [ ] CI/CD pipeline (GitHub Actions)

### Nice to Have

- [ ] Systemd units (`gwarden.service`, `gwarden@.service`)
- [ ] Bash/zsh completion
- [ ] Man pages
- [ ] Logging framework (structured logs)
- [ ] Metrics export (Prometheus format)

---

## Phase 2: Arch Linux Packaging (v0.3.0) 📦

**Goal:** Easy installation via AUR and official repos.

### Packaging

- [ ] **AUR Package**
  - [ ] Create PKGBUILD
  - [ ] `ghostwarden-git` (development)
  - [ ] `ghostwarden-bin` (pre-built)
  - [ ] `ghostwarden` (stable releases)

- [ ] **Debian Package** (via `cargo-deb`)
  - [ ] .deb for Ubuntu/Debian users
  - [ ] Systemd unit files included
  - [ ] Post-install hooks

- [ ] **Configuration Management**
  - [ ] Search path: `/etc/gwarden/`, `./`, `~/.config/gwarden/`
  - [ ] Default config generation: `gwarden init`
  - [ ] Example configs in `/usr/share/doc/gwarden/examples/`

### Distribution

- [ ] GitHub Releases with pre-built binaries
- [ ] Release notes automation
- [ ] Versioning strategy (SemVer)

---

## Phase 3: Proxmox VE Integration (v0.4.0) 🔥

**Goal:** First-class Proxmox support.

### Core Features

- [ ] **Proxmox API Client**
  - [ ] Authenticate with Proxmox API
  - [ ] List VMs/containers
  - [ ] Get VM network config
  - [ ] Update VM network devices

- [ ] **Import/Export**
  - [ ] `gwarden proxmox import` → topology.yaml
  - [ ] Discover existing vmbr* bridges
  - [ ] Import Proxmox firewall rules
  - [ ] Export Ghostwarden topology to Proxmox format

- [ ] **VM Attachment**
  - [ ] `gwarden proxmox attach --vm 100 --net vm_private`
  - [ ] Auto-detect Proxmox installation
  - [ ] Preserve existing VM config
  - [ ] Live attach (hot-plug support)

- [ ] **Cluster Support**
  - [ ] Multi-node topology sync
  - [ ] VXLAN overlay for VM migration
  - [ ] Corosync network protection (never manage)
  - [ ] Per-node configuration overrides

### Policy Profiles

- [ ] **Proxmox-Specific Profiles**
  - [ ] `proxmox-cluster` (Corosync, API, etc.)
  - [ ] `proxmox-storage` (Ceph, NFS, etc.)
  - [ ] `proxmox-backup` (PBS traffic)
  - [ ] `proxmox-dmz` (public-facing VMs)

### Documentation

- [ ] Proxmox migration guide
- [ ] Proxmox cluster setup example
- [ ] Firewall rule conversion tool
- [ ] Video tutorials

---

## Phase 4: Advanced Networking (v0.5.0) 🌐

**Goal:** Support complex enterprise setups.

### Features

- [ ] **VXLAN Overlays**
  - [ ] Multi-peer VXLAN support
  - [ ] FDB (forwarding database) management
  - [ ] VXLAN → bridge attachment (basic impl done)
  - [ ] EVPN for control plane (optional)

- [ ] **BGP Integration**
  - [ ] Announce subnets via FRR/BIRD
  - [ ] Dynamic routing for VXLAN
  - [ ] BFD for fast failover

- [ ] **QoS & Traffic Control**
  - [ ] Bandwidth limits per network
  - [ ] Traffic shaping (tc integration)
  - [ ] Priority queues

- [ ] **VPN Support**
  - [ ] WireGuard integration
  - [ ] Tailscale allowlist automation
  - [ ] IPsec support

### Policy Enhancements

- [ ] **Advanced Rules**
  - [ ] Time-based rules (cron-like)
  - [ ] Geo-blocking (IP lists)
  - [ ] Rate limiting (nftables meters)
  - [ ] Connection tracking limits

- [ ] **Integration Hooks**
  - [ ] CrowdSec ban list sync
  - [ ] Wazuh alert webhook → temp blocks
  - [ ] Graylog/ELK structured event shipping

---

## Phase 5: Observability (v0.6.0) 📊

**Goal:** Production-grade monitoring and visibility.

### Metrics

- [ ] **Prometheus Exporter** (`gwarden metrics serve`)
  - [ ] Bridge/interface counters (rx/tx, errors)
  - [ ] DHCP lease counts
  - [ ] nftables set sizes
  - [ ] Policy hit counters
  - [ ] Apply success/failure rates

- [ ] **eBPF Flow Monitoring**
  - [ ] Who-talks-to-who matrix
  - [ ] Top talkers by bandwidth
  - [ ] TUI flow visualization
  - [ ] Export to InfluxDB/Prometheus

### TUI Enhancements

- [ ] **Live Flow View**
  - [ ] eBPF-based flow tracking
  - [ ] Real-time packet counters
  - [ ] Connection state tracking

- [ ] **Interactive Operations**
  - [ ] Add/remove port forwards from TUI
  - [ ] Switch policy profiles
  - [ ] Trigger rollback
  - [ ] View detailed nftables rules

### Logging

- [ ] Structured logging (JSON)
- [ ] Log levels (debug, info, warn, error)
- [ ] Audit trail for all changes
- [ ] Integration with journald/syslog

---

## Phase 6: GUI & Automation (v0.7.0) 🖥️

**Goal:** Web GUI and automation capabilities.

### Web GUI

- [ ] **REST API**
  - [ ] FastAPI or Axum backend
  - [ ] JWT authentication
  - [ ] OpenAPI spec
  - [ ] WebSocket for live updates

- [ ] **Frontend**
  - [ ] React/Vue/Svelte SPA
  - [ ] Topology visualization (SVG/D3.js)
  - [ ] Policy editor
  - [ ] Real-time status dashboard
  - [ ] Mobile-responsive

### Daemon Mode

- [ ] **gwarden Daemon**
  - [ ] Systemd service
  - [ ] Watch topology files for changes
  - [ ] Auto-apply on change (opt-in)
  - [ ] Reconciliation loop (Kubernetes-style)

### Automation

- [ ] **Terraform Provider**
  - [ ] Manage Ghostwarden via IaC
  - [ ] `resource "ghostwarden_network"`
  - [ ] `resource "ghostwarden_policy"`

- [ ] **Ansible Modules**
  - [ ] `ghostwarden_network` module
  - [ ] `ghostwarden_apply` module
  - [ ] Example playbooks

---

## Phase 7: Enterprise Features (v1.0.0) 🏢

**Goal:** Enterprise-ready with HA and multi-tenancy.

### High Availability

- [ ] Active/passive failover
- [ ] State synchronization (etcd/Consul)
- [ ] Leader election
- [ ] VRRP/keepalived integration

### Multi-Tenancy

- [ ] Tenant isolation (network namespaces)
- [ ] Per-tenant quotas (bandwidth, VMs, etc.)
- [ ] RBAC for API access
- [ ] Audit logs per tenant

### Compliance

- [ ] SOC2 compliance helpers
- [ ] PCI-DSS network segmentation templates
- [ ] HIPAA-compliant policy profiles
- [ ] Audit report generation

---

## Community & Ecosystem

### Documentation

- [ ] Video tutorials
- [ ] Blog posts (architecture deep-dives)
- [ ] Conference talks (FOSDEM, KubeCon, etc.)
- [ ] Certification program

### Integrations

- [ ] Kubernetes CNI plugin
- [ ] Docker network driver
- [ ] OpenStack Neutron plugin
- [ ] Rancher integration

### Tooling

- [ ] VSCode extension (YAML autocomplete)
- [ ] Policy linter
- [ ] Topology validator CLI
- [ ] Migration tools (from UFW, firewalld, etc.)

---

## Long-Term Vision

**Ghostwarden aims to be:**

1. **The de-facto network orchestrator** for Arch/Gentoo/Void power users
2. **The Proxmox networking replacement** of choice
3. **A production-grade alternative** to Cilium/Calico for bare-metal Kubernetes
4. **The easiest way** to manage nftables declaratively

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for how to contribute to any of these roadmap items.

**Priority labels:**
- 🔥 **Critical** - Blocker for production use
- ⚡ **High** - Important for v1.0
- 📌 **Medium** - Nice to have
- 💭 **Low** - Future consideration

---

## Timeline Estimates

| Phase | Version | ETA | Focus |
|-------|---------|-----|-------|
| 1 | v0.2.0 | 1-2 months | Production hardening |
| 2 | v0.3.0 | 2-3 months | Packaging & distribution |
| 3 | v0.4.0 | 3-4 months | Proxmox integration |
| 4 | v0.5.0 | 4-6 months | Advanced networking |
| 5 | v0.6.0 | 6-8 months | Observability |
| 6 | v0.7.0 | 8-12 months | GUI & automation |
| 7 | v1.0.0 | 12-18 months | Enterprise features |

**Note:** These are rough estimates for a single maintainer. Community contributions can accelerate this significantly!

---

**Last Updated:** 2025-10-04
**Maintainer:** Christopher Kelley <ckelley@ghostkellz.sh>
