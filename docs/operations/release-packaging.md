# Release Packaging

Release assets live under [../../release/](../../release/).

## Current Assets

- install script
- systemd units
- shell completions
- man pages
- Arch/AUR packaging files
- sample daemon config

## Build

```bash
cargo build --release
```

## Install Binary

```bash
sudo install -Dm755 target/release/gwarden /usr/local/bin/gwarden
```

## Packaging Tasks

- Keep man pages aligned with clap commands.
- Regenerate shell completions after CLI changes.
- Validate systemd unit paths.
- Test Arch package build in a clean chroot.
