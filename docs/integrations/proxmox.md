# Proxmox VE

Ghostwarden can complement Proxmox by owning nftables policy, NAT, and declarative bridge topology while Proxmox continues to manage VM lifecycle and storage.

## Recommended Split

| Proxmox | Ghostwarden |
|---------|-------------|
| VM lifecycle | network topology |
| storage | nftables policy |
| cluster UI | port forwarding |
| VM config | DHCP/DNS for lab networks |

## Basic Flow

```bash
apt update
apt install -y cargo git nftables dnsmasq

git clone https://github.com/ghostkellz/ghostwarden.git
cd ghostwarden
cargo build --release
install -m 755 target/release/gwarden /usr/local/bin/gwarden
```

Disable the Proxmox firewall only if Ghostwarden is intended to own firewalling for the target networks:

```bash
systemctl disable --now pve-firewall
```

## Example Topology

```toml
version = 1

[interfaces]
uplink = "vmbr0"

[networks.public]
type = "bridge"
iface = "vmbr0"
policy_profile = "public-web"

[networks.vm_private]
type = "routed"
cidr = "10.99.0.0/24"
gw_ip = "10.99.0.1"
dhcp = true
masq_out = "vmbr0"
policy_profile = "routed-tight"

[networks.vm_private.dns]
enabled = true
zones = ["vm.internal"]
```

## Notes

- Do not let Proxmox firewall and Ghostwarden manage the same policy layer without a specific design.
- Keep a console session open during first apply.
- Review `vmbr*` bridge naming before Ghostwarden creates additional bridges.
