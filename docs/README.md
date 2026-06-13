# Ghostwarden Documentation

Ghostwarden manages Linux bridge, routed, VLAN, VXLAN, DHCP/DNS, nftables, and VM networking through a Rust CLI. This documentation is organized by workflow: install it, model a topology, apply it safely, observe the host, and troubleshoot failures.

## Start Here

- [Getting Started](getting-started/README.md)
- [Installation](getting-started/installation.md)
- [Configuration](getting-started/configuration.md)
- [Quick Start](getting-started/quick-start.md)
- [Production Readiness](operations/production-readiness.md)

## Documentation Map

| Section | Purpose |
|---------|---------|
| [getting-started/](getting-started/README.md) | Install Ghostwarden and run a first safe plan/apply cycle. |
| [architecture/](architecture/README.md) | Understand the workspace, planner, executor, nftables flow, and rollback model. |
| [reference/](reference/README.md) | CLI commands, topology TOML/YAML, policy profiles, and nftables behavior. |
| [operations/](operations/README.md) | Production checklist, observability, backups, rollback, and release packaging. |
| [integrations/](integrations/README.md) | Proxmox, libvirt, Docker coexistence, CrowdSec, and Wazuh integration notes. |
| [security/](security/README.md) | Threat model, policy hardening, privileges, and disclosure process. |
| [troubleshooting/](troubleshooting/README.md) | `gwarden doctor`, common failures, examples, and support capture. |

## Current Feature State

| Capability | State |
|------------|-------|
| Topology parsing | Implemented |
| Plan generation | Implemented |
| Bridge and VLAN helpers | Implemented |
| nftables ruleset generation | Implemented |
| DHCP/DNS via dnsmasq | Partial |
| Libvirt bridge attachment | Partial |
| TUI status dashboard | Implemented |
| Prometheus metrics | Implemented |
| Atomic apply | Planned |
| Persisted state database | Planned |

## Host Safety Notes

Ghostwarden changes live network state. Before applying a topology:

- Keep out-of-band access available.
- Run `gwarden net plan` until the output is understood.
- Back up current nftables, routes, addresses, and NetworkManager profiles.
- Start with a long rollback confirmation window.
- Avoid first-time production runs over a single SSH session.

## Repository Links

- Root README: [../README.md](../README.md)
- Security policy: [../SECURITY.md](../SECURITY.md)
- Contribution guide: [../CONTRIBUTING.md](../CONTRIBUTING.md)
- Task backlog: [../tasks/todo.md](../tasks/todo.md)
