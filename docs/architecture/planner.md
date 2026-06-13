# Planner

The planner translates a topology file into ordered actions. It is the safety boundary between the desired TOML/YAML topology and live network changes.

## Inputs

- `version`
- named host interfaces
- network definitions
- route/NAT details
- DHCP/DNS settings
- port forwards
- policy profile names

## Output

The planner returns actions such as:

- create bridge
- assign address
- configure VLAN/VXLAN
- generate nftables rules
- write dnsmasq configuration
- attach VM interfaces

## Expected Behavior

Planning should be deterministic. Running the same topology through `gwarden net plan` multiple times should produce the same proposed operations unless live host discovery is intentionally part of the plan.

## Near-Term Work

- Add plan serialization for review and testing.
- Separate desired-state diffing from action execution.
- Add integration tests against network namespaces.
- Make policy expansion visible in plan output.
