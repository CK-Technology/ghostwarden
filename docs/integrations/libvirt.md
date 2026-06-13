# Libvirt

Ghostwarden includes `virsh`-backed helpers for listing VMs and attaching VM interfaces to bridge networks.

## Host Setup

```bash
sudo systemctl enable --now libvirtd
sudo usermod -aG libvirt "$USER"
```

Log out and back in after changing groups.

## Commands

```bash
sudo gwarden vm list
sudo gwarden vm attach --vm devbox --net nat_dev
sudo gwarden vm attach --vm devbox --net nat_dev --tap tap-devbox-0
```

## Current Limits

- VM operations currently shell out to `virsh`.
- Advanced libvirt API integration is planned.
- Hot-plug and bandwidth options exist in code paths but need broader CLI exposure and tests.
