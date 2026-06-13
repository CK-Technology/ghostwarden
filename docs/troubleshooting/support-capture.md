# Support Capture

When reporting a Ghostwarden networking issue, collect enough host state to reproduce the failure without exposing secrets.

## Suggested Data

```bash
gwarden --version
sudo gwarden doctor
sudo gwarden net status
sudo gwarden net plan -f /path/to/topology.toml
ip addr
ip route
ip link show type bridge
sudo nft list ruleset
```

## Redact

- public IPs if sensitive
- MAC addresses if needed
- hostnames
- API tokens
- private domain names
- customer or client names

## Include

- distribution and kernel version
- whether the host uses NetworkManager, systemd-networkd, Proxmox, Docker, or libvirt
- whether the apply was local console or SSH
- expected result and actual result
