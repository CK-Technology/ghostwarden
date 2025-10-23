# Proxmox VE Integration

Ghostwarden can complement or replace Proxmox's built-in firewall with more flexible nftables-based policies.

## Overview

**Proxmox VE Firewall Limitations:**
- Uses iptables (legacy)
- Limited NAT/MASQUERADE support
- Per-VM configuration can be tedious
- No centralized policy profiles
- Difficult to manage complex VLAN setups

**Ghostwarden Benefits:**
- Modern nftables with better performance
- Centralized topology management
- Policy profiles for consistent security
- VLAN/VXLAN support
- Declarative YAML configuration
- Transactional apply with rollback

## Integration Strategy

### Option 1: Proxmox + Ghostwarden (Recommended)

Use Proxmox for VM management, Ghostwarden for networking.

**Proxmox Responsibilities:**
- VM lifecycle (start/stop/migrate)
- Storage management
- Web GUI for VM management

**Ghostwarden Responsibilities:**
- Network topology (bridges, VLANs)
- Firewall rules (nftables)
- DHCP/DNS
- Port forwarding
- Network policies

### Option 2: Ghostwarden-Only

Replace Proxmox networking entirely.

**Pros:**
- Complete control
- Consistent with non-Proxmox nodes
- Better for hybrid clusters

**Cons:**
- Lose Proxmox GUI network management
- Manual bridge configuration for VMs

## Setup Guide

### Prerequisites

```bash
# On Proxmox node
apt update
apt install -y cargo git nftables dnsmasq

# Build ghostwarden
git clone https://github.com/ghostkellz/ghostwarden.git
cd ghostwarden
cargo build --release
install -m 755 target/release/gwarden /usr/local/bin/
```

### 1. Disable Proxmox Firewall

```bash
# Stop Proxmox firewall
systemctl stop pve-firewall
systemctl disable pve-firewall

# Or configure to coexist (advanced)
# Edit /etc/pve/firewall/cluster.fw
# Set: enable: 0
```

### 2. Import Existing Proxmox Bridges

Create a topology that matches your Proxmox setup:

```yaml
# /etc/gwarden/topologies/proxmox-cluster.yaml
version: 1
interfaces:
  uplink: vmbr0  # Proxmox default bridge

networks:
  # Import existing vmbr0 as-is
  public:
    type: bridge
    iface: vmbr0
    policy_profile: public-web

  # Create new managed bridge for VMs
  vm_private:
    type: routed
    cidr: 10.99.0.0/24
    gw_ip: 10.99.0.1
    dhcp: true
    dns:
      enabled: true
      zones:
        - vm.internal
    masq_out: vmbr0
    policy_profile: routed-tight

  # VLAN for DMZ
  dmz:
    type: bridge
    iface: vmbr1
    vlan: 100
    policy_profile: dmz-restricted
```

### 3. Create Proxmox-Specific Policy Profiles

```yaml
# /etc/gwarden/policies/dmz-restricted.yaml
name: dmz-restricted
description: DMZ zone - limited access

default_action: drop

# Allow HTTP/HTTPS only
services:
  - protocol: tcp
    port: 80
  - protocol: tcp
    port: 443
  - protocol: udp
    port: 53

allowed_egress_cidrs:
  - 0.0.0.0/0

allowed_ingress_cidrs:
  - 10.0.0.0/8  # Only from private networks
```

### 4. Apply Ghostwarden Configuration

```bash
# Plan changes
gwarden net plan -f /etc/gwarden/topologies/proxmox-cluster.yaml

# Apply with rollback
gwarden net apply -f /etc/gwarden/topologies/proxmox-cluster.yaml --commit --confirm 60

# Verify
gwarden net status
```

### 5. Attach VMs to Ghostwarden Networks

**Option A: Manual (Proxmox GUI)**

1. Go to VM → Hardware → Network Device
2. Change Bridge to `vmbr-vm_private` (Ghostwarden-managed)
3. Restart VM networking

**Option B: Via Ghostwarden CLI**

```bash
# Not yet implemented - coming soon
# gwarden proxmox attach --vm 100 --net vm_private
```

**Option C: Via Proxmox CLI**

```bash
# Get VM config
qm config 100

# Update network bridge
qm set 100 --net0 virtio,bridge=br-vm_private

# Restart VM
qm reboot 100
```

## Proxmox + Ghostwarden Architecture

```
┌─────────────────────────────────────────┐
│         Proxmox VE Node                 │
├─────────────────────────────────────────┤
│                                         │
│  ┌──────────────┐  ┌──────────────┐   │
│  │   Proxmox    │  │  Ghostwarden │   │
│  │   (VM Mgmt)  │  │  (Networking)│   │
│  └──────────────┘  └──────────────┘   │
│         │                  │            │
│         │                  │            │
│  ┌──────▼──────────────────▼─────────┐ │
│  │     Linux Networking Stack        │ │
│  │  - Bridges (vmbr*, br-*)          │ │
│  │  - VLANs                           │ │
│  │  - nftables (Ghostwarden rules)   │ │
│  └───────────────────────────────────┘ │
│                  │                      │
└──────────────────┼──────────────────────┘
                   │
              ┌────▼────┐
              │ Physical│
              │   NIC   │
              └─────────┘
```

## Advanced: Proxmox Cluster Support

For multi-node Proxmox clusters:

```yaml
# /etc/gwarden/topologies/proxmox-cluster.yaml
version: 1

# Shared topology across all nodes
networks:
  # VXLAN for cross-node VM networking
  vm_overlay:
    type: vxlan
    vni: 100
    peers:
      - 192.168.1.10  # pve-node1
      - 192.168.1.11  # pve-node2
      - 192.168.1.12  # pve-node3
    bridge: br-vm-overlay

  # Corosync network (leave as-is)
  cluster:
    type: bridge
    iface: vmbr1
    policy_profile: cluster-only
```

Policy for cluster-only traffic:

```yaml
# /etc/gwarden/policies/cluster-only.yaml
name: cluster-only
description: Proxmox cluster communication only

default_action: drop

allowed_ingress_cidrs:
  - 192.168.1.0/24  # Cluster network

services:
  # Corosync
  - protocol: udp
    port: 5404
  - protocol: udp
    port: 5405
  # Proxmox API
  - protocol: tcp
    port: 8006
```

## Migrating from Proxmox Firewall

### 1. Export Proxmox Rules

```bash
# Proxmox firewall config location
cat /etc/pve/firewall/cluster.fw
cat /etc/pve/firewall/<vmid>.fw
```

### 2. Convert to Ghostwarden Policies

Manual conversion required. Example:

**Proxmox firewall rule:**
```
[RULES]
IN ACCEPT -p tcp -dport 22
IN ACCEPT -p tcp -dport 80
IN ACCEPT -p tcp -dport 443
```

**Ghostwarden equivalent:**
```yaml
# /etc/gwarden/policies/web-server.yaml
name: web-server
description: Web server with SSH

default_action: drop

services:
  - protocol: tcp
    port: 22
  - protocol: tcp
    port: 80
  - protocol: tcp
    port: 443

allowed_ingress_cidrs:
  - 0.0.0.0/0
```

### 3. Test Migration

1. Apply Ghostwarden config to test VM
2. Verify connectivity
3. Gradually migrate VMs
4. Monitor with `gwarden tui`

## Proxmox API Integration (Future)

Planned features:

```bash
# Auto-discover Proxmox VMs
gwarden proxmox discover

# Attach VM to network via API
gwarden proxmox attach --vm 100 --net vm_private

# Sync Ghostwarden topology to Proxmox
gwarden proxmox sync

# Import Proxmox network config
gwarden proxmox import > topology.yaml
```

## Monitoring & TUI

Use Ghostwarden TUI for live monitoring:

```bash
# Launch TUI
gwarden tui

# View:
# - All bridges (including Proxmox vmbr*)
# - nftables rules
# - DHCP leases
# - Press Tab to switch views
# - Press 'r' to refresh
# - Press 'q' to quit
```

## Troubleshooting

### VMs Can't Communicate

```bash
# Check bridge exists
ip link show br-vm_private

# Check nftables rules
nft list table inet gw

# Check VM bridge assignment
qm config <vmid> | grep net0
```

### Proxmox GUI Shows Wrong Network

Proxmox GUI may cache network info. Refresh or restart `pveproxy`:

```bash
systemctl restart pveproxy
```

### Corosync Issues After Apply

If cluster communication breaks:

1. Don't touch `vmbr1` (corosync network)
2. Rollback immediately
3. Add exception in topology

## Best Practices

1. **Never manage Proxmox's corosync bridge** (usually `vmbr1`)
2. **Test on non-production node first**
3. **Use separate bridges** for Ghostwarden vs Proxmox
4. **Enable rollback** with `--confirm 60` for safety
5. **Monitor cluster health** during migration
6. **Keep Proxmox firewall disabled** to avoid conflicts

## Next Steps

- [Proxmox VE Documentation](https://pve.proxmox.com/wiki/Network_Configuration)
- [VXLAN Setup Guide](../examples/vxlan-overlay.md)
- [Policy Profiles Reference](../reference/policy-profiles.md)
- [Production Checklist](../guides/best-practices.md)
