# Installation

Ghostwarden currently targets Arch Linux and lab hosts first. Other Linux distributions can work if they provide recent nftables, iproute2, dnsmasq, and Rust tooling.

## Requirements

- Linux kernel 5.10 or newer
- `nftables`
- `iproute2`
- `dnsmasq` when DHCP/DNS management is enabled
- root privileges for network changes
- Rust stable with edition 2024 support
- out-of-band access for first production-like applies

## Arch Linux Packages

```bash
sudo pacman -S --needed rust cargo git nftables dnsmasq iproute2 bridge-utils
```

Optional VM support:

```bash
sudo pacman -S --needed libvirt qemu virt-manager
sudo systemctl enable --now libvirtd
sudo usermod -aG libvirt "$USER"
```

## Build From Source

```bash
git clone https://github.com/ghostkellz/ghostwarden.git
cd ghostwarden
cargo build --release
sudo install -Dm755 target/release/gwarden /usr/local/bin/gwarden
gwarden --version
```

## Host Directories

```bash
sudo install -d /etc/gwarden
sudo install -d /etc/gwarden/policies
sudo install -d /var/lib/gwarden

sudo cp examples/ghostnet.toml /etc/gwarden/
sudo cp examples/policies/*.toml /etc/gwarden/policies/
```

## Services

Enable nftables:

```bash
sudo systemctl enable --now nftables
sudo nft list tables
```

Enable dnsmasq only if Ghostwarden-managed DHCP/DNS is part of the topology:

```bash
sudo systemctl enable dnsmasq
```

## Conflict Preparation

Ghostwarden should be the owner of networks it manages. Disable host firewalls that would compete for the same chains unless you have explicitly designed coexistence.

```bash
sudo systemctl disable --now ufw 2>/dev/null || true
sudo systemctl disable --now firewalld 2>/dev/null || true
```

For NetworkManager-managed hosts, mark Ghostwarden bridge prefixes unmanaged:

```bash
sudo tee /etc/NetworkManager/conf.d/99-ghostwarden.conf >/dev/null <<'EOF'
[keyfile]
unmanaged-devices=interface-name:br-*,interface-name:gw-*
EOF
sudo systemctl restart NetworkManager
```
