# Common Issues

## NAT Does Not Work

Check IP forwarding and nftables postrouting rules:

```bash
sysctl net.ipv4.ip_forward
sudo nft list ruleset | grep -i masquerade
sudo gwarden doctor nftables
```

Enable forwarding:

```bash
sudo sysctl -w net.ipv4.ip_forward=1
```

Persist it:

```bash
echo "net.ipv4.ip_forward = 1" | sudo tee /etc/sysctl.d/99-ghostwarden.conf
sudo sysctl --system
```

## Bridge Missing or Down

```bash
sudo gwarden doctor bridges
ip link show type bridge
ip addr show br-nat_dev
```

Load bridge modules:

```bash
sudo modprobe bridge
sudo modprobe br_netfilter
```

## Docker Conflict

```bash
sudo gwarden doctor docker
docker network inspect bridge
```

Move Docker address pools away from Ghostwarden CIDRs.

## dnsmasq Not Serving

```bash
sudo systemctl status dnsmasq
sudo journalctl -u dnsmasq -n 80
```

Confirm the generated config binds the expected bridge and that no other service owns port 53 on that interface.
