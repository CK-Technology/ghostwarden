# Configuration

Ghostwarden uses a topology file for desired network state and policy files for reusable security profiles. TOML is the default format; legacy YAML (`.yaml`/`.yml`) still loads.

## Paths

Recommended host layout:

```text
/etc/gwarden/
├── ghostnet.toml
└── policies/
    ├── l2-lan.toml
    ├── public-web.toml
    └── routed-tight.toml
```

Runtime state should live under `/var/lib/gwarden` as state persistence is implemented.

## Topology File

```toml
version = 1

[interfaces]
uplink = "enp6s0"

[networks.nat_dev]
type = "routed"
cidr = "10.33.0.0/24"
gw_ip = "10.33.0.1"
dhcp = true
masq_out = "enp6s0"
policy_profile = "routed-tight"

[networks.nat_dev.dns]
enabled = true
zones = ["dev.lan"]

[[networks.nat_dev.forwards]]
public = "0.0.0.0:4022/tcp"
dst = "10.33.0.10:22"
```

Existing YAML topologies continue to load; new configs should use TOML.

## Policy Profiles

Policy examples live in [../../examples/policies/](../../examples/policies/). Use profiles for repeatable network behavior instead of embedding one-off rules in every topology.

Recommended starting profiles:

- `routed-tight` for NAT networks with restrictive ingress.
- `public-web` for HTTP/HTTPS exposed workloads.
- `l2-lan` for trusted bridge/LAN segments.

## System Configuration

The sample [../../ghostwarden.toml.example](../../ghostwarden.toml.example) contains future daemon and integration settings for Proxmox, CrowdSec, Wazuh, nftables, and metrics. The current CLI path primarily uses topology and policy files.
