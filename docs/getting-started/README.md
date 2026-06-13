# Getting Started

Use this section to install Ghostwarden, prepare a host, and run the first safe plan/apply cycle.

## Pages

- [Installation](installation.md)
- [Configuration](configuration.md)
- [Quick Start](quick-start.md)

## Minimum Flow

```bash
cargo build --release
sudo install -Dm755 target/release/gwarden /usr/local/bin/gwarden

sudo gwarden net plan -f examples/ghostnet.toml
sudo gwarden doctor
sudo gwarden net apply -f examples/ghostnet.toml --commit --confirm 60
```

Use a VM or lab host for the first run.
