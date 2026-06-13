# Wazuh

Wazuh integration is planned for security event ingestion and enforcement through nftables or Proxmox-facing sets.

## Planned Configuration

See [../../ghostwarden.toml.example](../../ghostwarden.toml.example):

```toml
[wazuh]
api_url = "https://wazuh.example.com:55000"
api_user = "ghostwarden"
api_pass = "your-wazuh-password"
rules = ["5710", "5712"]
verify_ssl = true
```

## Tasks

- Implement authenticated API client.
- Map Wazuh rules to enforcement actions.
- Add dry-run and audit logging.
- Add per-source allowlists.
