# Topology Format

Topology files define the desired Ghostwarden-managed network state. TOML is the
default format; legacy YAML files still load (see [YAML compatibility](#yaml-compatibility)).

The canonical install path is `/etc/gwarden/ghostnet.toml`.

## Root Fields

| Field | Type | Description |
|-------|------|-------------|
| `version` | integer | topology schema version |
| `interfaces` | table | named host interfaces such as `uplink` |
| `networks` | table | managed routed, bridge, and VXLAN networks |

```toml
version = 1

[interfaces]
uplink = "enp6s0"
```

## Routed Network

```toml
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

## Bridge Network

```toml
[networks.br_work]
type = "bridge"
iface = "br-work"
vlan = 20
policy_profile = "l2-lan"
```

## VXLAN Network

```toml
[networks.overlay_lab]
type = "vxlan"
vni = 1200
bridge = "br-overlay"
peers = ["10.0.0.11", "10.0.0.12"]
```

## Port Forward Format

`public` uses `ip:port/protocol`. `dst` uses `ip:port`.

```toml
[[networks.nat_dev.forwards]]
public = "0.0.0.0:8443/tcp"
dst = "10.33.0.20:443"
```

## YAML Compatibility

Existing `.yaml`/`.yml` topologies continue to load via `Topology::from_file`
(format is selected by extension) and `Topology::from_yaml`. New configurations
should use TOML. The equivalent routed network in YAML:

```yaml
nat_dev:
  type: routed
  cidr: 10.33.0.0/24
  gw_ip: 10.33.0.1
  dhcp: true
  dns:
    enabled: true
    zones:
      - dev.lan
  masq_out: enp6s0
  forwards:
    - public: "0.0.0.0:4022/tcp"
      dst: "10.33.0.10:22"
  policy_profile: routed-tight
```
