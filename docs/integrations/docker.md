# Docker

Docker can coexist with Ghostwarden, but both tools affect container and host forwarding behavior.

## Diagnostics

```bash
sudo gwarden doctor docker
docker network ls
docker network inspect bridge
sudo iptables -L -n -v
sudo iptables -t nat -L -n -v
```

## Avoid Subnet Overlap

Set Docker address pools away from Ghostwarden routed networks:

```json
{
  "bip": "172.20.0.1/16",
  "default-address-pools": [
    {
      "base": "172.30.0.0/16",
      "size": 24
    }
  ]
}
```

Save as `/etc/docker/daemon.json`, then restart Docker.

## Firewall Ownership

Do not disable Docker iptables handling unless Ghostwarden or another tool fully replaces that policy for containers.
