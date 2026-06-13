# Production Readiness

Ghostwarden is early beta. Do not treat a host as production-ready until the checklist below is complete and rehearsed.

## Current State

| Area | State |
|------|-------|
| Topology planning | Implemented |
| Network status reporting | Implemented |
| Conflict detection | Implemented |
| TUI monitoring | Implemented |
| Policy profiles | Implemented |
| Libvirt VM helpers | Partial |
| VLAN support | Implemented |
| dnsmasq lifecycle | Partial |
| Atomic apply | Planned |
| State persistence | Planned |
| Structured logging | Planned |

## Pre-Deployment Checklist

- [ ] Confirm kernel, nftables, dnsmasq, and iproute2 are installed.
- [ ] Confirm root access and out-of-band console access.
- [ ] Disable or isolate competing firewalls such as UFW and firewalld.
- [ ] Configure NetworkManager to ignore Ghostwarden bridges.
- [ ] Back up addresses, routes, nftables, dnsmasq, and NetworkManager profiles.
- [ ] Test the topology in a VM or disposable lab host.
- [ ] Run `gwarden net plan` until the output is deterministic and understood.
- [ ] Apply with a long rollback window.
- [ ] Verify SSH, gateway, VM, DNS, DHCP, and outbound NAT behavior.
- [ ] Document a host runbook and rollback procedure.

## Backup Commands

```bash
ip addr show > ~/ghostwarden-ip-backup.txt
ip route show > ~/ghostwarden-route-backup.txt
sudo nft list ruleset > ~/ghostwarden-nft-backup.nft
sudo cp -a /etc/dnsmasq.d ~/ghostwarden-dnsmasq-backup
sudo cp -a /etc/NetworkManager/system-connections ~/ghostwarden-nm-backup
```

## First Apply

```bash
sudo gwarden net apply \
  -f /etc/gwarden/ghostnet.toml \
  --commit \
  --confirm 300
```

Use a five-minute window for first production-like runs.
