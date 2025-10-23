# GhostWarden Troubleshooting Guide

This guide helps you diagnose and fix common networking issues on Arch Linux and other distributions.

## Quick Start

GhostWarden includes a comprehensive diagnostics tool accessible via the `gwarden doctor` command:

```bash
# Run all diagnostics
sudo gwarden doctor

# Or run specific checks
sudo gwarden doctor nftables
sudo gwarden doctor docker
sudo gwarden doctor bridges
```

## Common Issues

### 1. Network Rules Not Applied

**Symptoms:**
- Traffic not being forwarded
- NAT not working
- Firewall rules not active

**Diagnosis:**
```bash
# Check nftables rules
sudo gwarden doctor nftables

# Manually inspect ruleset
sudo nft list ruleset

# Check if GhostWarden table exists
sudo nft list table inet gw
```

**Solutions:**

1. **Missing IP forwarding:**
```bash
# Check current setting
sysctl net.ipv4.ip_forward

# Enable temporarily
sudo sysctl -w net.ipv4.ip_forward=1

# Enable permanently (add to /etc/sysctl.conf)
echo "net.ipv4.ip_forward = 1" | sudo tee -a /etc/sysctl.conf
sudo sysctl -p
```

2. **nftables not running:**
```bash
# On Arch Linux
sudo systemctl enable nftables
sudo systemctl start nftables
```

3. **Rules not applied:**
```bash
# Re-apply configuration
sudo gwarden net apply --file ghostnet.yaml --commit
```

### 2. Bridge Issues

**Symptoms:**
- Bridges not created
- Interfaces not attached to bridges
- Bridge has no IP address

**Diagnosis:**
```bash
# Run bridge diagnostics
sudo gwarden doctor bridges

# List all bridges
ip link show type bridge

# Check specific bridge
ip addr show br-nat_dev

# Check bridge ports
ip link show master br-nat_dev
```

**Solutions:**

1. **Bridge module not loaded:**
```bash
# Load bridge module
sudo modprobe bridge

# Make it permanent
echo "bridge" | sudo tee /etc/modules-load.d/bridge.conf
```

2. **Bridge netfilter not enabled:**
```bash
# Load br_netfilter module
sudo modprobe br_netfilter

# Enable bridge netfilter
sudo sysctl -w net.bridge.bridge-nf-call-iptables=1
sudo sysctl -w net.bridge.bridge-nf-call-ip6tables=1

# Make permanent
cat <<EOF | sudo tee /etc/sysctl.d/99-bridge-nf.conf
net.bridge.bridge-nf-call-iptables = 1
net.bridge.bridge-nf-call-ip6tables = 1
net.bridge.bridge-nf-call-arptables = 1
EOF
```

3. **Bridge interface down:**
```bash
# Bring up bridge
sudo ip link set br-nat_dev up
```

### 3. Docker Networking Conflicts

**Symptoms:**
- Docker containers can't reach internet
- Subnet conflicts
- iptables/nftables rule conflicts

**Diagnosis:**
```bash
# Check Docker networking
sudo gwarden doctor docker

# Check Docker networks
docker network ls
docker network inspect bridge

# Check for iptables conflicts
sudo iptables -L -n -v
sudo iptables -t nat -L -n -v
```

**Solutions:**

1. **Subnet overlap:**

Check your Docker daemon configuration and GhostWarden topology for overlapping subnets:

```bash
# View Docker default subnet
docker network inspect bridge | grep Subnet

# Configure custom Docker subnet in /etc/docker/daemon.json
sudo mkdir -p /etc/docker
cat <<EOF | sudo tee /etc/docker/daemon.json
{
  "bip": "172.20.0.1/16",
  "default-address-pools": [
    {
      "base": "172.30.0.0/16",
      "size": 24
    }
  ]
}
EOF

# Restart Docker
sudo systemctl restart docker
```

2. **iptables/nftables coexistence:**

Docker uses iptables by default, which can coexist with nftables:

```bash
# Check if iptables rules are present
sudo iptables -L DOCKER -n

# If using nftables-native Docker (experimental)
# Add to /etc/docker/daemon.json
{
  "iptables": false
}

# WARNING: Only disable if you're managing firewall rules manually
```

3. **Docker bridge not working:**
```bash
# Check docker0 interface
ip addr show docker0

# Restart Docker daemon
sudo systemctl restart docker
```

### 4. NAT/Masquerading Not Working

**Symptoms:**
- Internal network can't reach internet
- Outbound traffic blocked
- Ping works but HTTP doesn't

**Diagnosis:**
```bash
# Check NAT rules
sudo gwarden doctor nftables

# View postrouting chain
sudo nft list chain inet gw postrouting

# Check masquerade rules
sudo nft list ruleset | grep -i masq
```

**Solutions:**

1. **Check masq_out interface:**

Verify that your topology specifies the correct outbound interface:

```yaml
networks:
  nat_dev:
    routed:
      cidr: 10.0.100.0/24
      gw_ip: 10.0.100.1
      masq_out: eth0  # <- Make sure this matches your actual interface
```

2. **Verify interface name:**
```bash
# List all interfaces
ip link show

# Common interface names:
# - eth0, ens3, enp0s3 (Ethernet)
# - wlan0, wlp2s0 (WiFi)
# - Use the one with internet access
```

3. **Test connectivity:**
```bash
# From the bridge network
ping -I br-nat_dev 8.8.8.8

# From a VM/container
# Ensure default route points to bridge IP
ip route
```

### 5. Kernel Module Issues

**Required modules:**
- `nf_tables` - Core nftables support
- `nf_nat` - NAT functionality
- `nf_conntrack` - Connection tracking
- `br_netfilter` - Bridge + netfilter integration

**Diagnosis:**
```bash
# Check loaded modules
lsmod | grep -E "nf_|bridge"

# Run comprehensive check
sudo gwarden doctor
```

**Solutions:**
```bash
# Load all required modules
sudo modprobe nf_tables
sudo modprobe nf_nat
sudo modprobe nf_conntrack
sudo modprobe br_netfilter

# Make permanent
cat <<EOF | sudo tee /etc/modules-load.d/gwarden.conf
nf_tables
nf_nat
nf_conntrack
br_netfilter
EOF
```

### 6. Package Update Issues (Arch Linux)

**Symptoms:**
- Build fails after system update
- Missing dependencies
- Incompatible package versions

**Solutions:**

1. **Update Rust toolchain:**
```bash
rustup update
```

2. **Clean and rebuild:**
```bash
cargo clean
cargo build --release
```

3. **Check for missing system packages:**
```bash
# Install build dependencies
sudo pacman -S base-devel git nftables iproute2 bridge-utils

# Optional but recommended
sudo pacman -S docker libvirt
```

## Advanced Diagnostics

### Packet Tracing

Use nftables trace to debug rule matching:

```bash
# Add trace rule to beginning of chain
sudo nft add rule inet gw forward meta nftrace set 1

# Monitor trace output
sudo nft monitor trace

# Remove trace rule when done
sudo nft flush chain inet gw forward
sudo gwarden net apply --commit  # Reapply config
```

### Connection Tracking

Check connection tracking table:

```bash
# View active connections
sudo conntrack -L

# Check NAT connections
sudo conntrack -L -p tcp --dport 80

# Monitor new connections
sudo conntrack -E
```

### tcpdump Analysis

Capture traffic on bridge interfaces:

```bash
# Capture on bridge
sudo tcpdump -i br-nat_dev -n

# Capture specific traffic
sudo tcpdump -i br-nat_dev -n host 10.0.100.50

# Save to file for analysis
sudo tcpdump -i br-nat_dev -w /tmp/capture.pcap
```

### Checking Rule Counters

View nftables rule hit counters:

```bash
# List rules with counters
sudo nft list table inet gw -a

# Watch counters in real-time
watch -n1 'sudo nft list table inet gw'
```

## Getting Help

If you're still experiencing issues:

1. **Run full diagnostics:**
   ```bash
   sudo gwarden doctor > /tmp/gwarden-diagnostics.txt
   ```

2. **Gather system information:**
   ```bash
   uname -a
   ip link
   ip addr
   ip route
   sudo nft list ruleset
   ```

3. **Check logs:**
   ```bash
   # Kernel messages
   sudo dmesg | tail -50

   # System logs
   sudo journalctl -xe -u docker
   sudo journalctl -xe -u systemd-networkd
   ```

4. **Report issue:**
   - Open an issue at: https://github.com/ghostkellz/ghostwarden
   - Include diagnostics output
   - Describe what you were trying to do
   - Include your topology YAML (sanitize sensitive IPs)

## Best Practices

1. **Always test changes before committing:**
   ```bash
   # Dry run
   sudo gwarden net plan --file ghostnet.yaml

   # Apply with confirmation window
   sudo gwarden net apply --confirm 30
   ```

2. **Use version control for topology files:**
   ```bash
   git init
   git add ghostnet.yaml
   git commit -m "Initial network topology"
   ```

3. **Document your network:**
   Add comments to your topology YAML explaining design decisions

4. **Regular backups:**
   ```bash
   # Backup current ruleset
   sudo nft list ruleset > /etc/gwarden/backup-$(date +%Y%m%d).nft

   # Backup topology
   cp ghostnet.yaml ghostnet.yaml.backup
   ```

5. **Monitor after changes:**
   ```bash
   # Use TUI to monitor
   sudo gwarden tui

   # Or watch status
   watch -n2 'sudo gwarden net status'
   ```

## Performance Tuning

### Increase conntrack limits

For high-traffic scenarios:

```bash
# Increase connection tracking limit
sudo sysctl -w net.netfilter.nf_conntrack_max=262144

# Make permanent
echo "net.netfilter.nf_conntrack_max = 262144" | sudo tee -a /etc/sysctl.conf
```

### Optimize bridge forwarding

```bash
# Disable netfilter on bridges if not needed
sudo sysctl -w net.bridge.bridge-nf-call-iptables=0
sudo sysctl -w net.bridge.bridge-nf-call-ip6tables=0
```

### Enable jumbo frames

For VM-to-VM traffic:

```bash
# Increase MTU on bridge
sudo ip link set br-nat_dev mtu 9000
```

---

**Last Updated:** 2025-10-05
**Maintainer:** Christopher Kelley <ckelley@ghostkellz.sh>
