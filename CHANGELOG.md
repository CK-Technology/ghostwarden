# Changelog

All notable changes to Ghostwarden are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## 2026-06-13

### Added
- `gwarden net state` and `gwarden net state-clear` commands for inspecting and
  clearing the persisted apply state (`--json` output; `--clear` guarded by `--confirm`).
- `transaction_id` on rollback snapshots so `rollback.json` correlates with `applied-state.json`.
- Rollback preview: `gwarden net rollback` without `--execute` enumerates each operation.
- Root-gated integration tests for netlink (bridge/address/VLAN) and nftables
  (apply/snapshot/restore), plus policy-profile parser tests.
- Architecture diagrams (apply, rollback, nftables pipeline, Proxmox/libvirt topology).

### Changed
- TOML is now the default topology and policy format; `/etc/gwarden/ghostnet.toml`
  is the canonical path. Legacy YAML (`.yaml`/`.yml`) still loads.
- Documentation migrated to TOML-first; `topology-yaml.md` renamed to `topology-format.md`.
- Man pages and shell completions updated for the new commands and corrected
  `--confirm` (plain seconds) and `-f` file flags.

### Fixed
- VLAN rollback now uses the VLAN delete path instead of bridge deletion.
