# Quick Start

This flow validates a topology and applies it with rollback protection.

## 1. Build

```bash
cargo build --release
```

## 2. Review the Example

```bash
sed -n '1,160p' examples/ghostnet.toml
```

Change `interfaces.uplink` and any `masq_out` values to match the host.

## 3. Run Diagnostics

```bash
sudo target/release/gwarden doctor
```

Fix nftables, Docker, bridge, and sysctl warnings before applying.

## 4. Plan

```bash
sudo target/release/gwarden net plan -f examples/ghostnet.toml
```

Review every bridge, address, nftables rule, and dnsmasq change.

## 5. Apply With Rollback

```bash
sudo target/release/gwarden net apply -f examples/ghostnet.toml --commit --confirm 60
```

During the confirmation window, verify SSH, gateway reachability, VM traffic, and DNS/DHCP if enabled.

## 6. Inspect State

```bash
sudo target/release/gwarden net status
sudo nft list ruleset
ip link show type bridge
```
