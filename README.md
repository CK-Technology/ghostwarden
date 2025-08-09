# Ghostwarden

<div align="center">
  <img src="assets/icons/ghostwarden-icon.png" alt="Ghostwarden Icon" width="128" height="128">

**Proxmox SDN + Cluster Firewall Enforcement, Powered by Zig**

![zig](https://img.shields.io/badge/Built%20with-Zig-yellow?logo=zig)
![proxmox](https://img.shields.io/badge/Platform-Proxmox%20VE-orange?logo=proxmox)
![sdn](https://img.shields.io/badge/SDN-Enabled-blue)
![firewall](https://img.shields.io/badge/Firewall-Cluster%20%26%20Zone-red)
![crowdsec](https://img.shields.io/badge/Integration-CrowdSec-4B7BBE?logo=crowdsource)
![wazuh](https://img.shields.io/badge/Integration-Wazuh-005B94)

</div>

---

## Overview

**Ghostwarden** is a high-performance Proxmox VE security bouncer built in Zig, designed to integrate seamlessly with **CrowdSec** and **Wazuh** to enforce bans and policies across:

* **Proxmox VE Cluster Firewall**
* **Proxmox SDN zones**
* **Local nftables/ipsets**
* Optional NGINX/IP set includes

Ghostwarden pulls ban decisions from CrowdSec's LAPI and/or Wazuh events, then applies them instantly at the Proxmox and network level. It's SDN-aware, cluster-aware, and built for speed.

---

## ✨ Features

* **Cluster & SDN Enforcement**

  * Sync bans to Proxmox cluster IP sets
  * Apply per-zone SDN firewall rules
* **CrowdSec Integration**

  * Pull decisions from LAPI in real time
  * Enforce community and custom scenarios
* **Wazuh Integration**

  * Parse alerts and trigger firewall actions
* **Fast Local Mitigation**

  * Update nftables/ipsets instantly for zero-delay blocking
* **Cluster-Aware**

  * Distribute bans across all Proxmox nodes
* **Configurable TTLs & Whitelists**

  * Auto-expire bans
  * Protect management networks and trusted IPs

---

## 📦 Components

* **ghostwardend** — main daemon, runs sync loops and applies bans
* **collectors/** — Wazuh alert parser, local log tailers
* **bouncers/**

  * `pve` — Proxmox API client for IPSet + SDN rules
  * `nft` — local nftables set updater
  * `nginx` — optional map/include updater
* **compat/**

  * `crowdsec_lapi` — CrowdSec LAPI client

---

## 🚀 Quick Start (Planned)

```bash
# Install Ghostwarden
zig build -Drelease-safe

# Run the daemon with a config file
ghostwardend --config /etc/ghostwarden/config.toml
```

**Example Config**

```toml
[pve]
api_url = "https://proxmox.example.com:8006/api2/json"
token_id = "root@pam!ghostwarden"
token_secret = "<token>"
ipset_name = "zekebanned"

[crowdsec]
lapi_url = "http://crowdsec-lapi:8080"
api_key = "<lapi-api-key>"

[wazuh]
api_url = "https://wazuh.example.com"
api_user = "gw-bouncer"
api_pass = "<password>"
```

---

## 🗺 Roadmap

* [ ] MVP: CrowdSec LAPI → PVE IPSet sync
* [ ] Add Wazuh alert → IPSet rules
* [ ] Per-zone SDN rules
* [ ] Local nftables fast-path bouncer
* [ ] Whitelist/TTL management
* [ ] Prometheus `/metrics` endpoint

---

## 📜 License

MIT

