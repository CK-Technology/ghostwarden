# Policy Hardening

Use policy profiles to make network intent explicit and repeatable.

## Baseline Recommendations

- Default to deny for routed lab networks.
- Expose only required ingress ports.
- Keep management networks separate from public service networks.
- Avoid wildcard source CIDRs for administrative services.
- Use port forwards sparingly and document each one.
- Keep DNS and DHCP scoped to intended bridges.

## Review Checklist

- [ ] Each network has a policy profile.
- [ ] Public forwards are intentional and documented.
- [ ] SSH is not exposed broadly without compensating controls.
- [ ] Docker and libvirt subnets do not overlap with Ghostwarden CIDRs.
- [ ] nftables output matches the expected policy.
