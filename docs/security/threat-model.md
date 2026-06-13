# Threat Model

Ghostwarden has high host impact because it manipulates networking, firewall, NAT, DHCP/DNS, and VM attachment state.

## Assets

- host management access
- nftables rulesets
- VM and container network isolation
- DHCP/DNS configuration
- topology and policy files
- future integration credentials

## Primary Risks

- operator lockout from incorrect firewall or route changes
- accidental exposure from broad port forwards
- subnet overlap with Docker, libvirt, or Proxmox networks
- malicious or unreviewed topology/policy changes
- leaked API credentials for CrowdSec, Wazuh, or Proxmox

## Controls

- review plans before apply
- enforce rollback windows
- restrict write access to `/etc/gwarden`
- keep secrets out of topology files
- prefer least-privilege integration tokens
- log applied changes and keep backups
