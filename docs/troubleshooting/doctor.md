# Doctor

`gwarden doctor` runs diagnostics for common host-networking failures.

## Commands

```bash
sudo gwarden doctor
sudo gwarden doctor all
sudo gwarden doctor nftables
sudo gwarden doctor docker
sudo gwarden doctor bridges
```

## Areas Checked

- nftables availability and rules
- bridge interfaces and kernel support
- Docker networking conflicts
- sysctl and module assumptions

## Useful Follow-Up Commands

```bash
sudo nft list ruleset
ip link show type bridge
ip addr
ip route
docker network ls
```
