# Rollback

Rollback is required because Ghostwarden changes live host networking and can break the management path to the machine.

## Current Model

`gwarden net apply --commit --confirm <seconds>` applies the requested topology and gives the operator a confirmation window. If the change is not confirmed, rollback cleanup should reverse the applied bridge, address, and nftables state.

## Operator Rules

- Use out-of-band console access for first-time applies.
- Use a longer confirmation window for remote hosts.
- Test SSH, gateway reachability, VM traffic, DNS, and DHCP during the window.
- Keep a host network backup before changing production-like systems.

## Required Hardening

- Persist applied state snapshots.
- Make action execution transactional.
- Track which resources were created by Ghostwarden.
- Add network namespace integration tests for rollback order.
- Add a clear dry-run rollback preview.
