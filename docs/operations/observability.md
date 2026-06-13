# Observability

Ghostwarden exposes host networking visibility through CLI status, a TUI, diagnostics, and a Prometheus metrics endpoint.

## CLI Status

```bash
sudo gwarden net status
```

Status collects bridge, nftables, and DHCP lease information where available.

## TUI

```bash
sudo gwarden tui
```

The TUI currently shows:

- bridges
- nftables tables
- DHCP leases

## Metrics

```bash
gwarden metrics serve --addr :9138
curl http://127.0.0.1:9138/metrics
```

Metrics include bridge status, nftables counts, DHCP lease counts, apply success/failure counters, and rollback counters.

## Planned Work

- Add structured logs with `tracing`.
- Add support bundle generation.
- Add redaction for interface names, MAC addresses, and host identifiers.
- Add JSON output for status and doctor commands.
