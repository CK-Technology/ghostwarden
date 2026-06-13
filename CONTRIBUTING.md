# Contributing

Ghostwarden is a Rust networking project with real host impact. Contributions should be small, reviewable, and tested against the failure modes they touch.

## Development Setup

```bash
git clone https://github.com/ghostkellz/ghostwarden.git
cd ghostwarden
cargo build --workspace
cargo check --workspace
```

Useful host tools:

- `nft`
- `ip`
- `dnsmasq`
- `virsh`
- Docker or libvirt for coexistence testing
- a VM or network namespace test environment

## Before Sending Changes

```bash
cargo fmt --all
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features
```

Run `cargo audit` when dependency changes are included.

## Change Guidelines

- Keep host-changing behavior explicit and reviewable.
- Add dry-run or plan output for new execution paths.
- Prefer structured parsing over string scraping when a stable API exists.
- Keep docs and examples in sync with CLI changes.
- Add tests for planner, parser, ruleset, and rollback behavior.
- Do not hide privileged side effects behind convenience commands.

## Documentation Style

- Use lowercase descriptive Markdown filenames.
- Add folder-level `README.md` index files.
- Keep root README focused on status, quick start, docs links, and project shape.
- Put detailed operational material under `docs/`.

## Security-Sensitive Changes

Open a private security report instead of a public pull request if the change demonstrates an exploitable vulnerability. See [SECURITY.md](SECURITY.md).
