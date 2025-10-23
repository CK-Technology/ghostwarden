# Production Readiness Checklist

Before deploying Ghostwarden on your Arch system, complete this checklist.

## ⚠️ Current Status: **BETA** (v0.1.1)

Ghostwarden v0.1.1 has production-critical features implemented but **NOT yet battle-tested** in production.

**Recent improvements:**
- Complete policy profile → nftables integration
- Full rollback cleanup implementation
- SSH connectivity monitoring
- Docker and libvirt compatibility enhancements

### What Works

- ✅ Topology planning (dry-run mode)
- ✅ Network status reporting
- ✅ Conflict detection
- ✅ TUI for monitoring
- ✅ Policy profile system
- ✅ Libvirt VM management
- ✅ VLAN support

### What's Partially Complete

- ⚠️ **dnsmasq integration** - Config generation works, restart may need tweaking
- ⚠️ **Error handling** - Happy path works, edge cases not fully handled

### Recently Completed (v0.1.1)

- ✅ **Policy profiles → nftables integration** - Fully wired up with complete ruleset generation
- ✅ **Rollback cleanup** - Properly deletes bridges, addresses, nftables tables in reverse order
- ✅ **SSH connectivity check** - TCP-based connectivity verification implemented
- ✅ **Enhanced MASQUERADING** - Full NAT rules with stateful firewall support
- ✅ **Docker compatibility** - Docker bridge detection and network management helpers
- ✅ **Libvirt enhancements** - Interface models, MAC generation, bandwidth limiting, hot-plug support
- ✅ **Bridge utilities** - Get bridge info, list members, attach/detach interfaces, set MTU

### What's Missing for Production

- ❌ **Atomic apply** - No transaction guarantees yet
- ❌ **Comprehensive testing** - Unit tests exist, integration tests needed
- ❌ **State persistence** - No way to query "what did I apply last time?"
- ❌ **Structured logging** - Only println! debugging, no structured logs
- ❌ **API integrations** - Tailscale, Wazuh, CrowdSec, Prometheus exporters planned

---

## Pre-Deployment Checklist

### 1. System Requirements ✅

- [ ] Arch Linux (or compatible distro)
- [ ] Linux kernel 5.10+ (check: `uname -r`)
- [ ] nftables installed (check: `nft --version`)
- [ ] dnsmasq installed (check: `dnsmasq --version`)
- [ ] Root access available
- [ ] Out-of-band access (IPMI, console, etc.) in case of lockout

### 2. Conflict Resolution ✅

Run conflict detection:

```bash
sudo gwarden net apply -f examples/ghostnet.yaml
# (Don't use --commit yet, just check conflicts)
```

**Required actions:**

- [ ] UFW disabled: `sudo systemctl disable --now ufw`
- [ ] firewalld disabled: `sudo systemctl disable --now firewalld`
- [ ] NetworkManager configured to ignore bridges:
  ```bash
  cat /etc/NetworkManager/conf.d/99-ghostwarden.conf
  # Should contain:
  [keyfile]
  unmanaged-devices=interface-name:br-*
  ```

### 3. Backup Current Network Config 🔥

**CRITICAL:** Save your current network state!

```bash
# Save current IP configuration
ip addr show > ~/network-backup-$(date +%Y%m%d).txt

# Save routing table
ip route show >> ~/network-backup-$(date +%Y%m%d).txt

# Save nftables rules (if any)
sudo nft list ruleset > ~/nftables-backup-$(date +%Y%m%d).nft

# Save dnsmasq config
sudo cp -r /etc/dnsmasq.d ~/dnsmasq-backup-$(date +%Y%m%d)

# Save NetworkManager connections (if used)
sudo cp -r /etc/NetworkManager/system-connections ~/nm-backup-$(date +%Y%m%d)
```

### 4. Test in VM First 🧪

**DO NOT test on production host first!**

1. Set up Arch VM (libvirt, VirtualBox, etc.)
2. Install Ghostwarden in VM
3. Apply configuration
4. Verify:
   - Can still SSH to VM
   - VMs on bridge can talk
   - DHCP works
   - Internet access works (if using NAT)
5. Test rollback by waiting for timeout

### 5. Plan Your Topology 📋

- [ ] Identify uplink interface (e.g., `enp6s0`)
- [ ] Choose bridge names (e.g., `br-nat_dev`)
- [ ] Choose subnet (e.g., `10.33.0.0/24`)
- [ ] Plan port forwards
- [ ] Select policy profile

**Example minimal topology:**

```yaml
version: 1
interfaces:
  uplink: enp6s0  # YOUR INTERFACE HERE

networks:
  mgmt:
    type: routed
    cidr: 10.50.0.0/24
    gw_ip: 10.50.0.1
    dhcp: true
    dns:
      enabled: true
      zones:
        - mgmt.lan
    masq_out: enp6s0
    policy_profile: routed-tight
```

### 6. Dry-Run Multiple Times 🔍

```bash
# Run plan 3+ times to ensure deterministic output
sudo gwarden net plan -f /etc/gwarden/topologies/my-network.yaml
sudo gwarden net plan -f /etc/gwarden/topologies/my-network.yaml
sudo gwarden net plan -f /etc/gwarden/topologies/my-network.yaml
```

**Check for:**
- [ ] Same plan every time (idempotent)
- [ ] No unexpected actions
- [ ] Bridge names correct
- [ ] CIDR ranges correct

### 7. Enable Out-of-Band Access 🚨

**BEFORE applying, ensure you have console access:**

- [ ] Physical access to machine
- [ ] OR IPMI/iDRAC/iLO console
- [ ] OR KVM-over-IP
- [ ] OR serial console

**DO NOT apply over SSH without out-of-band access!**

### 8. First Apply with Long Timeout ⏰

```bash
# Use 5-minute timeout for first apply
sudo gwarden net apply \
  -f /etc/gwarden/topologies/my-network.yaml \
  --commit \
  --confirm 300

# You have 5 minutes to test connectivity
# If SSH still works, press ENTER to confirm
# If SSH breaks, rollback happens automatically
```

**During timeout, test:**

- [ ] Can still ping gateway
- [ ] Can still SSH to host
- [ ] VMs can reach internet (if NAT)
- [ ] DHCP working (if enabled)
- [ ] DNS working (if enabled)

### 9. Verify Applied State ✅

```bash
# Check network status
sudo gwarden net status

# Check bridges
ip link show type bridge

# Check nftables
sudo nft list ruleset | grep -A 20 "table inet gw"

# Check dnsmasq
sudo systemctl status dnsmasq
sudo journalctl -u dnsmasq -n 50

# Test connectivity
ping 10.50.0.1  # Your gateway
```

### 10. Document Your Setup 📝

Create a runbook:

```bash
# /etc/gwarden/RUNBOOK.md

## Rollback Procedure

If Ghostwarden breaks networking:

1. Delete bridges:
   sudo ip link del br-mgmt

2. Delete nftables table:
   sudo nft delete table inet gw

3. Restore old network config:
   sudo systemctl restart NetworkManager
   # OR
   sudo netctl restart <profile>

4. Check connectivity:
   ping 8.8.8.8

## Emergency Console Commands

(Use IPMI/serial console if SSH is down)

systemctl stop dnsmasq
ip link del br-mgmt
nft delete table inet gw
systemctl restart NetworkManager
```

---

## Known Issues & Workarounds

### ~~Issue 1: Rollback Doesn't Clean Up~~ ✅ FIXED in v0.1.1

**Status:** RESOLVED - Rollback now properly cleans up all resources in reverse order.

### ~~Issue 2: Policy Profiles Not Applied to nftables~~ ✅ FIXED in v0.1.1

**Status:** RESOLVED - Policy profiles now generate complete nftables rulesets with stateful firewall rules.

### Issue 3: dnsmasq May Fail to Restart

**Problem:** `systemctl restart dnsmasq` may fail if config is invalid.

**Workaround:**
```bash
# Test config before applying
sudo dnsmasq --test --conf-file=/etc/dnsmasq.d/gw-*.conf

# View errors
sudo journalctl -u dnsmasq -n 50
```

**Fix ETA:** v0.2.0 (add validation)

### Issue 4: No State Persistence

**Problem:** Can't query "what's currently applied?"

**Workaround:** Keep a copy of your applied topology YAML.

**Fix ETA:** v0.3.0

---

## Emergency Recovery

### If Locked Out via SSH

1. **Use console access** (IPMI/KVM/physical)

2. **Check network interfaces:**
   ```bash
   ip addr show
   ip link show
   ```

3. **Delete Ghostwarden bridges:**
   ```bash
   ip link del br-mgmt
   ip link del br-nat_dev
   # ... repeat for all gw bridges
   ```

4. **Restore old network config:**
   ```bash
   systemctl restart NetworkManager
   # OR
   netctl restart profile-name
   ```

5. **Verify connectivity:**
   ```bash
   ping 8.8.8.8
   ```

6. **Review what went wrong:**
   ```bash
   journalctl -xe
   gwarden net status
   ```

### If Kernel Panic / Boot Failure

(Unlikely, but just in case)

1. **Boot into rescue mode**
2. **Mount root filesystem**
3. **Remove Ghostwarden configs:**
   ```bash
   rm -rf /etc/gwarden/
   ```
4. **Reboot**

---

## Recommended First Deployment

### Scenario: Single Development Host

**Goal:** NAT network for VMs with DHCP/DNS.

**Topology:**

```yaml
version: 1
interfaces:
  uplink: enp6s0

networks:
  dev:
    type: routed
    cidr: 10.99.0.0/24
    gw_ip: 10.99.0.1
    dhcp: true
    dns:
      enabled: true
      zones:
        - dev.lan
    masq_out: enp6s0
    policy_profile: routed-tight
```

**Steps:**

1. Test in VM first
2. Apply on host with `--confirm 300`
3. Verify VMs can communicate
4. Attach one VM to test
5. Monitor with `gwarden tui`
6. Document your setup

---

## When to Use Ghostwarden (v0.1.0)

### ✅ Good Use Cases

- Development workstations with VMs
- Home labs with libvirt
- Single-node Proxmox for testing
- Learning nftables declaratively

### ❌ **DO NOT USE** Yet

- Production servers (not hardened)
- Multi-node clusters (no HA)
- Public-facing services (not audited)
- Anything without console access

---

## Getting Help

If something goes wrong:

1. **Check logs:** `journalctl -xe`
2. **Check Ghostwarden status:** `gwarden net status`
3. **Review rollback procedure** above
4. **File an issue:** https://github.com/ghostkellz/ghostwarden/issues

Include:
- Ghostwarden version: `gwarden --version`
- OS: `uname -a`
- Topology YAML (redact sensitive info)
- Error output
- `journalctl -xe` output

---

## Next Steps

Once you've deployed successfully:

- [ ] Set up monitoring (Prometheus + TUI)
- [ ] Create more policy profiles
- [ ] Test rollback scenario
- [ ] Document your runbook
- [ ] Join the community (GitHub Discussions)

**Remember:** v0.1.0 is ALPHA. Expect bugs. Have backups. Use console access.

---

**Production-Ready ETA:** v0.2.0 (see [ROADMAP.md](../ROADMAP.md))

**Last Updated:** 2025-10-04
