# Policy Profiles

Policy profiles are reusable network security presets. They keep topology files readable and make repeated intent explicit.

## Included Examples

| Profile | Purpose |
|---------|---------|
| `routed-tight` | default deny posture for routed/NAT networks |
| `public-web` | expose HTTP/HTTPS-style workloads |
| `l2-lan` | trusted local bridge behavior |

Example files live under [../../examples/policies/](../../examples/policies/).

## Recommended Practice

- Use a profile for every network.
- Keep permissive profiles visibly named.
- Review profile changes like firewall changes.
- Prefer narrow ingress and explicit egress.

## Planned Reference Work

- Document the full profile schema.
- Add generated examples for nftables output.
- Add validation errors for unsupported fields.
