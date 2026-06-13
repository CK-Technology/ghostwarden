# Packaging

This directory contains source-controlled packaging and install inputs.

## Contents

```text
packaging/
├── aur/                  # Arch package recipe and install hook
├── configs/              # default daemon configuration
├── systemd/              # systemd unit files
└── install.sh            # manual install/uninstall helper
```

Generated completions and man pages live under [../release/](../release/).

## Defaults

- Topology: `/etc/gwarden/ghostnet.toml`
- Policies: `/etc/gwarden/policies/*.toml`
- Daemon config: `/etc/gwarden/daemon.toml`
- State: `/var/lib/gwarden`
- Logs: `/var/log/gwarden`

TOML is the preferred operator-facing format. YAML remains loadable for older configurations.

## Manual Install

```bash
cargo build --release
sudo packaging/install.sh
```

To build during install:

```bash
sudo packaging/install.sh --build
```

To uninstall:

```bash
sudo packaging/install.sh --uninstall
```

## Arch Package

```bash
cd packaging/aur
makepkg -si
```

Before publishing, update `pkgver`, regenerate `.SRCINFO`, and replace `sha256sums=('SKIP')` with real release checksums.
