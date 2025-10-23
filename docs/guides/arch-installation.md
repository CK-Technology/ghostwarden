# Installing Ghostwarden on Arch Linux

## Prerequisites

### Required Packages

```bash
# Core networking
sudo pacman -S nftables dnsmasq bridge-utils iproute2

# Optional (for VM management)
sudo pacman -S libvirt qemu virt-manager

# Build dependencies
sudo pacman -S rust cargo git
```

### System Requirements

- **Kernel:** Linux 5.10+ (for modern nftables features)
- **Architecture:** x86_64 or aarch64
- **Privileges:** Root access required for network operations

## Installation

### Option 1: Build from Source

```bash
# Clone repository
git clone https://github.com/ghostkellz/ghostwarden.git
cd ghostwarden

# Build release binary
cargo build --release

# Install to /usr/local/bin
sudo install -m 755 target/x86_64-unknown-linux-gnu/release/gwarden /usr/local/bin/

# Verify installation
gwarden --version
```

### Option 2: Install from AUR (Coming Soon)

```bash
yay -S ghostwarden-git
```

## Configuration

### 1. Create Config Directory

```bash
sudo mkdir -p /etc/gwarden/{policies,topologies}
sudo mkdir -p /var/lib/gwarden
```

### 2. Copy Example Configuration

```bash
# Copy example topology
sudo cp examples/ghostnet.yaml /etc/gwarden/topologies/

# Copy policy profiles
sudo cp examples/policies/*.yaml /etc/gwarden/policies/
```

### 3. Disable Conflicting Services

**Important:** Ghostwarden conflicts with UFW, firewalld, and may conflict with NetworkManager.

```bash
# Check for conflicts
gwarden net apply -f /etc/gwarden/topologies/ghostnet.yaml

# Disable UFW (if installed)
sudo systemctl disable --now ufw

# Disable firewalld (if installed)
sudo systemctl disable --now firewalld

# Configure NetworkManager to ignore ghostwarden bridges
sudo tee /etc/NetworkManager/conf.d/99-ghostwarden.conf <<EOF
[keyfile]
unmanaged-devices=interface-name:br-*,interface-name:gw-*
EOF

sudo systemctl restart NetworkManager
```

### 4. Enable nftables

```bash
# Enable nftables service
sudo systemctl enable nftables
sudo systemctl start nftables

# Verify nftables is running
sudo nft list tables
```

### 5. Configure dnsmasq

```bash
# Enable dnsmasq
sudo systemctl enable dnsmasq

# dnsmasq will be configured automatically by gwarden
# Configs will be written to /etc/dnsmasq.d/gw-*.conf
```

## First Network Setup

### 1. Review Example Topology

Edit `/etc/gwarden/topologies/ghostnet.yaml`:

```yaml
version: 1
interfaces:
  uplink: enp6s0  # Change to your actual interface name

networks:
  nat_dev:
    type: routed
    cidr: 10.33.0.0/24
    gw_ip: 10.33.0.1
    dhcp: true
    dns:
      enabled: true
      zones:
        - dev.lan
    masq_out: enp6s0  # Your uplink interface
    forwards:
      - public: "0.0.0.0:4022/tcp"
        dst: "10.33.0.10:22"
    policy_profile: routed-tight
```

### 2. Plan Changes (Dry Run)

```bash
sudo gwarden net plan -f /etc/gwarden/topologies/ghostnet.yaml
```

### 3. Apply Configuration

```bash
# Apply with 30s rollback timeout
sudo gwarden net apply -f /etc/gwarden/topologies/ghostnet.yaml --commit --confirm 30

# Press ENTER to confirm within 30 seconds
# Or wait for auto-rollback
```

### 4. Verify Network

```bash
# Check network status
sudo gwarden net status

# List bridges
ip link show type bridge

# Check nftables rules
sudo nft list ruleset | grep gw

# Test connectivity
ping 10.33.0.1
```

## Libvirt Integration

### 1. Configure libvirt

```bash
# Start libvirt
sudo systemctl enable --now libvirtd

# Add your user to libvirt group
sudo usermod -a -G libvirt $USER

# Re-login for group to take effect
```

### 2. Attach VM to Ghostwarden Network

```bash
# List VMs
sudo gwarden vm list

# Attach VM to network
sudo gwarden vm attach --vm myvm --net nat_dev

# Or with custom tap name
sudo gwarden vm attach --vm myvm --net nat_dev --tap vm-myvm-0
```

## Troubleshooting

### Network Not Working

```bash
# Check bridge status
ip addr show br-nat_dev

# Check forwarding
cat /proc/sys/net/ipv4/conf/br-nat_dev/forwarding

# Check nftables rules
sudo nft list table inet gw

# Check dnsmasq
sudo systemctl status dnsmasq
sudo journalctl -u dnsmasq -n 50
```

### Rollback Failed Apply

```bash
# If apply failed, manually delete bridge
sudo ip link del br-nat_dev

# Delete nftables table
sudo nft delete table inet gw

# Stop dnsmasq
sudo systemctl stop dnsmasq
```

### NetworkManager Interference

If NetworkManager keeps taking over bridges:

```bash
# Add to /etc/NetworkManager/NetworkManager.conf
[main]
plugins=keyfile

[keyfile]
unmanaged-devices=interface-name:br-*

sudo systemctl restart NetworkManager
```

## Next Steps

- [CLI Reference](../reference/cli.md)
- [Policy Profiles](../reference/policy-profiles.md)
- [TUI Guide](../guides/tui.md)
- [Best Practices](../guides/best-practices.md)

## Uninstallation

```bash
# Stop services
sudo systemctl stop dnsmasq

# Remove bridges
sudo ip link del br-nat_dev

# Remove nftables rules
sudo nft delete table inet gw

# Remove binary
sudo rm /usr/local/bin/gwarden

# Remove configs (optional)
sudo rm -rf /etc/gwarden
```
