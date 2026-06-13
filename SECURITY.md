# Security Policy

## Supported Versions

Ghostwarden is pre-1.0. Security fixes are handled on the main development branch unless a release branch is explicitly announced.

## Reporting a Vulnerability

Do not open a public issue for a suspected vulnerability.

Send a private report to:

- `ckelley@ghostkellz.sh`

Include:

- affected commit, tag, or package
- host distribution and kernel version
- reproduction steps
- expected and observed behavior
- logs or command output with secrets redacted
- whether the issue can cause lockout, firewall bypass, privilege escalation, credential exposure, or remote code execution

## Scope

Security-sensitive areas include:

- nftables rule generation and application
- rollback failure or management lockout
- topology and policy parsing
- command execution paths
- libvirt and Proxmox integration
- CrowdSec and Wazuh credentials or API calls
- metrics or diagnostics leaking secrets
- packaging or install scripts that modify privileged paths

## Disclosure Process

1. The report is acknowledged as soon as practical.
2. The issue is reproduced and severity is assigned.
3. A fix is prepared privately when needed.
4. A patched release or commit is published.
5. Public notes are written with enough detail for operators to assess exposure without publishing exploit instructions unnecessarily.

## Operator Guidance

- Treat topology and policy files as privileged configuration.
- Keep `/etc/gwarden` writable only by trusted administrators.
- Review `gwarden net plan` before every apply.
- Maintain out-of-band access for hosts where Ghostwarden manages the primary network path.
- Store integration tokens with least privilege and rotate them after suspected exposure.
