# CrowdSec

CrowdSec integration is planned around ingesting ban decisions and syncing them into nftables sets or Proxmox ipsets.

## Planned Configuration

See [../../ghostwarden.toml.example](../../ghostwarden.toml.example):

```toml
[crowdsec]
lapi_url = "http://crowdsec.example.com:8080"
api_key = "your-crowdsec-api-key"
scenarios = ["crowdsecurity/ssh-bf", "crowdsecurity/http-bf"]
poll_interval_seconds = 30
```

## Tasks

- Implement LAPI client.
- Add nftables set sync.
- Add whitelist enforcement.
- Add metrics for decisions applied and expired.
