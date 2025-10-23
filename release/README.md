# GhostWarden Release Artifacts

This directory contains all packaging and release materials for GhostWarden.

## Directory Structure

```
release/
├── aur/                    # Arch User Repository package
│   ├── PKGBUILD           # AUR build script
│   ├── .SRCINFO           # AUR metadata
│   └── ghostwarden.install # Post-install script
├── systemd/               # Systemd service units
│   ├── gwarden.service    # Main daemon service
│   └── gwarden-metrics.service # Metrics exporter service
├── configs/               # Configuration files
│   └── daemon.toml        # Daemon configuration
├── completions/           # Shell completion scripts
│   ├── gwarden.bash       # Bash completion
│   ├── gwarden.zsh        # Zsh completion
│   └── gwarden.gsh        # Gshell completion
├── man/                   # Manual pages
│   ├── gwarden.1          # Main manual
│   ├── gwarden-net.1      # Network management
│   ├── gwarden-vm.1       # VM management
│   ├── gwarden-forward.1  # Port forwarding
│   ├── gwarden-policy.1   # Policy management
│   ├── gwarden-metrics.1  # Metrics server
│   ├── gwarden-doctor.1   # Diagnostics
│   └── gwarden-graph.1    # Visualization
└── README.md              # This file
```

## Building Packages

### Arch Linux (AUR)

```bash
cd release/aur
makepkg -si
```

Or install from AUR (once published):
```bash
yay -S ghostwarden
# or
paru -S ghostwarden
```

### From Source

```bash
cargo build --release
sudo install -Dm755 target/release/gwarden /usr/bin/gwarden
```

## Installation

### Arch Linux

```bash
yay -S ghostwarden
```

### Manual Installation

```bash
# Build
cargo build --release

# Install binary
sudo install -Dm755 target/release/gwarden /usr/bin/gwarden

# Install systemd units
sudo install -Dm644 release/systemd/gwarden.service /usr/lib/systemd/system/
sudo install -Dm644 release/systemd/gwarden-metrics.service /usr/lib/systemd/system/

# Install configuration
sudo mkdir -p /etc/ghostwarden/policies
sudo install -Dm644 examples/ghostnet.yaml /etc/ghostwarden/
sudo install -Dm644 release/configs/daemon.toml /etc/ghostwarden/
sudo install -Dm644 examples/policies/*.yaml /etc/ghostwarden/policies/

# Install completions
sudo install -Dm644 release/completions/gwarden.bash /usr/share/bash-completion/completions/gwarden
sudo install -Dm644 release/completions/gwarden.zsh /usr/share/zsh/site-functions/_gwarden
sudo install -Dm644 release/completions/gwarden.gsh /usr/share/gsh/completions/gwarden.gsh

# Install man pages
sudo install -Dm644 release/man/*.1 -t /usr/share/man/man1/

# Create state directory
sudo mkdir -p /var/lib/ghostwarden
```

## Post-Installation

### Enable Services

```bash
# Enable metrics server
sudo systemctl enable --now gwarden-metrics.service

# Enable network daemon (optional)
sudo systemctl enable gwarden.service
```

### Configure

1. Edit topology:
```bash
sudo nano /etc/ghostwarden/ghostnet.yaml
```

2. Preview changes:
```bash
gwarden net plan
```

3. Apply configuration:
```bash
sudo gwarden net apply --commit --confirm 30s
```

## Shell Completions

### Bash

Completions are automatically loaded if you installed via package manager.

Manual activation:
```bash
source /usr/share/bash-completion/completions/gwarden
```

### Zsh

Add to `~/.zshrc`:
```zsh
fpath=(/usr/share/zsh/site-functions $fpath)
autoload -Uz compinit && compinit
```

### Gshell (gsh)

Completions are automatically loaded from `/usr/share/gsh/completions/`.

Features:
- Smart command completion
- Dynamic network/policy name completion
- Argument validation
- Contextual hints
- Command aliases
- Keyboard shortcuts (Ctrl-G-*)

## Systemd Services

### gwarden.service

Main daemon that watches topology file and applies changes automatically.

```bash
# Enable and start
sudo systemctl enable --now gwarden.service

# Check status
sudo systemctl status gwarden.service

# View logs
journalctl -u gwarden.service -f
```

### gwarden-metrics.service

Prometheus metrics exporter on port 9138.

```bash
# Enable and start
sudo systemctl enable --now gwarden-metrics.service

# Test metrics
curl http://localhost:9138/metrics

# Check logs
journalctl -u gwarden-metrics.service -f
```

## Configuration Files

### /etc/ghostwarden/ghostnet.yaml

Main topology configuration. Defines:
- Networks (bridges, NAT, VLANs)
- Port forwards
- DHCP/DNS settings
- Policy profiles

Example:
```yaml
version: 1
networks:
  nat_dev:
    type: routed
    cidr: 10.33.0.0/24
    dhcp: true
    dns: true
    masq_out: enp6s0
    forwards:
      - { public: ":4022/tcp", dst: "10.33.0.10:22" }
```

### /etc/ghostwarden/daemon.toml

Daemon configuration. Controls:
- Metrics server settings
- Rollback behavior
- Logging configuration
- Integration settings

## Manual Pages

View documentation:
```bash
man gwarden              # Main manual
man gwarden-net          # Network management
man gwarden-vm           # VM management
man gwarden-forward      # Port forwarding
man gwarden-policy       # Policy management
man gwarden-metrics      # Metrics server
man gwarden-doctor       # Diagnostics
man gwarden-graph        # Visualization
```

## Troubleshooting

### Run Diagnostics

```bash
sudo gwarden doctor          # All checks
sudo gwarden doctor nftables # Check firewall
sudo gwarden doctor docker   # Check Docker
sudo gwarden doctor bridges  # Check bridges
```

### Check Status

```bash
gwarden net status     # Network status
systemctl status gwarden.service
systemctl status gwarden-metrics.service
journalctl -u gwarden.service -n 50
```

### Rollback

If something goes wrong:
```bash
sudo gwarden net rollback
```

Or use the automatic rollback:
```bash
sudo gwarden net apply --commit --confirm 30s
# Wait 30 seconds, press ENTER to confirm
# Or wait for automatic rollback
```

## Uninstallation

### Arch Linux

```bash
yay -R ghostwarden
```

### Manual

```bash
# Stop services
sudo systemctl stop gwarden.service gwarden-metrics.service
sudo systemctl disable gwarden.service gwarden-metrics.service

# Remove binary
sudo rm /usr/bin/gwarden

# Remove systemd units
sudo rm /usr/lib/systemd/system/gwarden.service
sudo rm /usr/lib/systemd/system/gwarden-metrics.service

# Remove completions
sudo rm /usr/share/bash-completion/completions/gwarden
sudo rm /usr/share/zsh/site-functions/_gwarden
sudo rm /usr/share/gsh/completions/gwarden.gsh

# Remove man pages
sudo rm /usr/share/man/man1/gwarden*.1

# Optional: Remove configuration and state
sudo rm -rf /etc/ghostwarden
sudo rm -rf /var/lib/ghostwarden
```

## Release Checklist

Before creating a release:

- [ ] Update version in Cargo.toml
- [ ] Update version in PKGBUILD
- [ ] Update version in man pages
- [ ] Update CHANGELOG.md
- [ ] Run tests: `cargo test --workspace`
- [ ] Build release: `cargo build --release`
- [ ] Generate checksum: `sha256sum target/release/gwarden`
- [ ] Update PKGBUILD sha256sums
- [ ] Test AUR package: `makepkg -si`
- [ ] Create git tag: `git tag -a v0.X.Y -m "Release v0.X.Y"`
- [ ] Push tag: `git push origin v0.X.Y`
- [ ] Create GitHub release with binary
- [ ] Update AUR repository

## Support

- GitHub Issues: https://github.com/ghostkellz/ghostwarden/issues
- Documentation: https://github.com/ghostkellz/ghostwarden
- Email: ckelley@ghostkellz.sh

## License

MIT © 2025 CK Technology / GhostKellz
