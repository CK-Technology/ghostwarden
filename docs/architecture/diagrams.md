# Diagrams

Visual references for the apply, rollback, nftables, and VM-topology flows. See
[Overview](overview.md) for the high-level crate pipeline.

## Apply Flow

`gwarden net apply` plans, snapshots, executes, then holds a confirmation window
before committing. If the confirm window expires or a probe fails, it auto-rolls back.

```mermaid
flowchart TD
    LOAD["load topology\nTopology::from_file"] --> PLAN["planner\nordered Plan"]
    PLAN --> VALID["validator\nreject unsafe plan"]
    VALID --> SNAP

    subgraph SNAP["snapshot (pre-execute)"]
        R1["rollback.json\nRollbackRecord + nft snapshots"]
        R2["applied-state.json\nApplyState + OwnedResource[]"]
    end

    SNAP --> EXEC
    subgraph EXEC["execute actions"]
        E1["bridge / VLAN"] --> E2["address"]
        E2 --> E3["nftables apply"]
        E3 --> E4["dnsmasq config"]
    end

    EXEC --> CONFIRM{"--confirm\nwindow"}
    CONFIRM -->|"probe ok / user confirms"| COMMIT["commit\nkeep state"]
    CONFIRM -->|"probe fails / timeout"| RB["auto-rollback\nreplay RollbackOps"]
    RB -.-> CLEAR["clear rollback.json\n+ applied-state.json"]
```

## Rollback Flow

`gwarden net rollback` (no `--execute`) previews the operations; with `--execute`
it replays each `RollbackOp` in order, then clears the record.

```mermaid
flowchart TD
    LOADR["load rollback.json\nRollbackRecord"] --> PREVIEW["preview ops\ndescribe each RollbackOp"]
    PREVIEW --> GATE{"--execute?"}
    GATE -->|"no"| DONE["print preview only"]
    GATE -->|"yes"| REPLAY

    subgraph REPLAY["replay in order"]
        O1["DeleteVlan"] --> O2["DeleteBridge"]
        O2 --> O3["RemoveAddress"]
        O3 --> O4["RestoreNft\n(snapshot or delete table)"]
        O4 --> O5["DeleteDnsmasqConfig"]
    end

    REPLAY --> CLR["clear rollback.json\n+ applied-state.json"]
```

## nftables Pipeline

Topology plus the referenced policy profile resolve into a generated ruleset that is
applied with a snapshot and can be diffed against the live host.

```mermaid
flowchart LR
    TOPO["topology\nnetworks + forwards"] --> RESOLVE
    PROF["policy_profile\nrouted-tight / public-web / l2-lan"] --> RESOLVE

    RESOLVE["policy resolve\nservices + egress CIDRs"] --> GEN

    subgraph GEN["ruleset generation"]
        NAT["NAT chain\nMASQUERADE / DNAT"]
        FWD["forward chain\nstateful forwarding"]
        FILT["filter chain\ndefault drop + allows"]
    end

    GEN --> APPLY["apply with snapshot\nnft -j"]
    APPLY -.->|"compare"| DIFF["gwarden net diff\nvs live ruleset"]
```

## Proxmox / libvirt VM Topology

Ghostwarden owns the bridges and VLANs on each node; VMs attach their tap interfaces
to those bridges via `virsh`, while the uplink carries routed/NAT traffic out.

```mermaid
flowchart TB
    UP["uplink iface\nenp6s0 / vmbr0"] --> GW

    subgraph GW["gwarden-managed (per node)"]
        BR1["bridge br-work\nVLAN 20 · l2-lan"]
        BR2["routed nat_dev\n10.33.0.0/24 · routed-tight"]
    end

    subgraph PVE["Proxmox / libvirt host"]
        VM1["VM devbox\ntap0"] -.->|"virsh attach"| BR2
        VM2["VM lab\ntap1"] -.->|"virsh attach"| BR1
    end

    GW --> NFT["nftables\nNAT + forward + filter"]
    NFT --> UP
```
