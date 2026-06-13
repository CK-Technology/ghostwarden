# nftables

Ghostwarden uses nftables as the primary firewall and NAT backend. See the
[nftables pipeline diagram](../architecture/diagrams.md#nftables-pipeline) for how
topology and policy data become a live ruleset.

## Responsibilities

- Generate rulesets from topology and policy profile data.
- Configure MASQUERADE for routed networks.
- Configure DNAT/SNAT-style port forwarding.
- Maintain stateful forwarding behavior.
- Report live table, chain, and rule status.

## Inspecting Live State

```bash
sudo nft list ruleset
sudo nft list tables
sudo nft list table inet gw
```

## Diffing Desired State

```bash
sudo gwarden net diff -f /etc/gwarden/ghostnet.toml
```

## Coexistence

Avoid multiple tools owning the same nftables tables or firewall policy. Docker may still use iptables compatibility rules; run `gwarden doctor docker` to inspect common conflicts.
