# Release Outputs

This directory is for generated or distribution-facing release outputs.

## Contents

```text
release/
├── completions/
│   ├── gwarden.bash
│   └── gwarden.zsh
└── man/
    ├── gwarden.1
    ├── gwarden-doctor.1
    ├── gwarden-forward.1
    ├── gwarden-metrics.1
    ├── gwarden-net.1
    ├── gwarden-policy.1
    └── gwarden-vm.1
```

Package recipes, service units, installer scripts, and default config files live in [../packaging/](../packaging/).

## Refresh Checklist

Before tagging a release:

- Regenerate bash and zsh completions from the current CLI.
- Regenerate man pages from the current CLI.
- Run `cargo fmt --all`.
- Run `cargo clippy --workspace --all-targets --all-features`.
- Run `cargo test --workspace`.
- Run `cargo audit`.
- Validate [../packaging/aur/PKGBUILD](../packaging/aur/PKGBUILD) in a clean Arch chroot.
