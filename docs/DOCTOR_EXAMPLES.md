# GhostWarden Doctor Command Examples

The `gwarden doctor` command provides comprehensive network diagnostics.

## Usage

```bash
# Run all diagnostics (recommended)
sudo gwarden doctor

# Run specific checks
sudo gwarden doctor nftables
sudo gwarden doctor docker
sudo gwarden doctor bridges
```

## Example Output

### Full Diagnostic Report

```bash
$ sudo gwarden doctor

🩺 Running comprehensive network diagnostics...

╔═══════════════════════════════════════════════════════════════╗
║            GhostWarden Troubleshooting Report                ║
╚═══════════════════════════════════════════════════════════════╝

━━━ nftables/iptables ━━━

ℹ️ INFO nftables ruleset found
  Ruleset size: 2847 bytes

ℹ️ INFO GhostWarden table found
  Found 'table inet gw' in ruleset

ℹ️ INFO NAT masquerade found
  MASQUERADE rule is configured

ℹ️ INFO NAT output interface
  Masquerading via interface: eth0

ℹ️ INFO Kernel module: nf_tables
  Core nftables support is loaded

ℹ️ INFO Kernel module: nf_nat
  NAT support is loaded

ℹ️ INFO Kernel module: nf_conntrack
  Connection tracking is loaded

⚠️ WARN Missing kernel module: br_netfilter
  Bridge netfilter support is not loaded
  💡 Suggestion: Load the module with: modprobe br_netfilter
  🔧 Fix: sudo modprobe br_netfilter

ℹ️ INFO IP forwarding enabled
  net.ipv4.ip_forward = 1

━━━ Docker Networking ━━━

ℹ️ INFO Docker daemon running
  Docker daemon is accessible

ℹ️ INFO Docker iptables integration enabled
  Docker is managing iptables rules
  💡 Suggestion: This may interact with nftables - ensure proper rule precedence

ℹ️ INFO Docker networks found
  Found 3 Docker network(s)

ℹ️ INFO Docker bridge (docker0) exists
  Default Docker bridge interface is present

ℹ️ INFO Docker bridge IP
  docker0: 172.17.0.1/16

ℹ️ INFO Docker bridge subnet
  Default bridge subnet: 172.17.0.0/16

⚠️ WARN Common Docker subnet detected
  Using default 172.17.0.0/16 - may conflict with VPNs or corporate networks
  💡 Suggestion: Consider configuring a custom subnet in /etc/docker/daemon.json

ℹ️ INFO Docker iptables chains found
  Docker is managing its own iptables chains

ℹ️ INFO DOCKER-USER chain available
  You can add custom rules to DOCKER-USER chain
  💡 Suggestion: Use DOCKER-USER for custom firewall rules that should apply to Docker containers

━━━ Bridge Configuration ━━━

ℹ️ INFO iproute2 tools available
  The 'ip' command is available for network configuration

ℹ️ INFO Bridges found
  Found 3 bridge interface(s): docker0, br-nat_dev, br-vm_private

ℹ️ INFO Bridge docker0 status
  State: UP

ℹ️ INFO Bridge docker0 MTU
  MTU: 1500

ℹ️ INFO Bridge docker0 IPv4
  IP: 172.17.0.1/16

ℹ️ INFO Bridge br-nat_dev status
  State: UP

ℹ️ INFO Bridge br-nat_dev MTU
  MTU: 1500

ℹ️ INFO Bridge br-nat_dev IPv4
  IP: 10.0.100.1/24

ℹ️ INFO Bridge br-nat_dev ports
  2 port(s): veth0, tap-vm1

ℹ️ INFO GhostWarden bridges detected
  Found 2 GhostWarden bridge(s): br-nat_dev, br-vm_private

⚠️ WARN Cannot read net.bridge.bridge-nf-call-iptables
  br_netfilter module may not be loaded
  💡 Suggestion: Load module: modprobe br_netfilter
  🔧 Fix: sudo modprobe br_netfilter

━━━ Summary ━━━
  ⚠️  3 warning(s) found
```

## Specific Diagnostics

### nftables Only

```bash
$ sudo gwarden doctor nftables

🔍 Checking nftables/iptables configuration...

ℹ️ INFO nftables ruleset found
  Ruleset size: 2847 bytes

ℹ️ INFO GhostWarden table found
  Found 'table inet gw' in ruleset

ℹ️ INFO NAT masquerade found
  MASQUERADE rule is configured

...
```

### Docker Only

```bash
$ sudo gwarden doctor docker

🔍 Checking Docker networking...

ℹ️ INFO Docker daemon running
  Docker daemon is accessible

ℹ️ INFO Docker bridge (docker0) exists
  Default Docker bridge interface is present

...
```

### Bridges Only

```bash
$ sudo gwarden doctor bridges

🔍 Checking bridge configuration...

ℹ️ INFO Bridges found
  Found 3 bridge interface(s): docker0, br-nat_dev, br-vm_private

ℹ️ INFO GhostWarden bridges detected
  Found 2 GhostWarden bridge(s): br-nat_dev, br-vm_private

...
```

## Common Scenarios

### Scenario 1: Fresh Installation

After installing GhostWarden on a fresh Arch Linux system:

```bash
$ sudo gwarden doctor

# Typical issues found:
# - br_netfilter module not loaded
# - IP forwarding disabled
# - No GhostWarden bridges (expected)

# Fix:
sudo modprobe br_netfilter
sudo sysctl -w net.ipv4.ip_forward=1
```

### Scenario 2: After Package Update

If network breaks after `pacman -Syu`:

```bash
$ sudo gwarden doctor

# Check for:
# - Missing kernel modules (may need reboot)
# - nftables service status
# - Bridge interfaces still present

# Common fix:
sudo reboot  # If kernel was updated
```

### Scenario 3: NAT Not Working

Traffic from internal network can't reach internet:

```bash
$ sudo gwarden doctor nftables

# Look for:
# - IP forwarding disabled
# - No masquerade rules
# - Wrong output interface

# Example fix:
sudo sysctl -w net.ipv4.ip_forward=1
sudo gwarden net apply --commit
```

### Scenario 4: Docker Conflicts

Docker containers can't communicate with GhostWarden networks:

```bash
$ sudo gwarden doctor

# Check for:
# - Overlapping subnets
# - iptables/nftables conflicts
# - Bridge netfilter settings

# Fix subnet overlap:
# Edit /etc/docker/daemon.json
{
  "bip": "172.20.0.1/16"
}
# Restart Docker
sudo systemctl restart docker
```

## Integration with Other Commands

### Before Applying Configuration

```bash
# Check system health first
sudo gwarden doctor

# Then plan changes
sudo gwarden net plan

# Apply
sudo gwarden net apply --commit
```

### After System Changes

```bash
# After kernel update
sudo reboot
sudo gwarden doctor
sudo gwarden net status

# After network changes
sudo gwarden doctor
sudo gwarden net apply --commit
```

### Debugging Failed Apply

```bash
# Apply fails
sudo gwarden net apply --commit
# Error: ...

# Run diagnostics
sudo gwarden doctor

# Fix issues identified
# Then retry
sudo gwarden net apply --commit
```

## Diagnostic Levels

The doctor command reports findings at different severity levels:

- **ℹ️ INFO**: Informational, no action needed
- **⚠️ WARN**: Potential issue, may need attention
- **❌ ERROR**: Problem that should be fixed
- **🔥 CRITICAL**: Blocking issue, must fix

## Exit Codes

- `0`: All checks passed or only warnings
- `1`: Errors or critical issues found

Use in scripts:

```bash
if sudo gwarden doctor; then
    echo "System healthy, proceeding with deployment"
    sudo gwarden net apply --commit
else
    echo "Issues detected, please review"
    exit 1
fi
```

---

**Last Updated:** 2025-10-05
