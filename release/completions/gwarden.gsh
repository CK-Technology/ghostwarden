# GhostWarden completion for gsh (gshell)
# This file provides intelligent command completion for gwarden in gshell

# Main gwarden command completion
complete gwarden {
    # Top-level commands
    commands = [
        "net"       # Network topology management
        "vm"        # Virtual machine network management
        "forward"   # Port forwarding management
        "policy"    # Security policy management
        "metrics"   # Prometheus metrics server
        "doctor"    # Network diagnostics
        "graph"     # Topology visualization
        "tui"       # Terminal UI dashboard
        "help"      # Help information
    ]

    # Subcommand completions
    net = {
        commands = ["plan" "apply" "status" "rollback"]
        flags = ["--commit" "--confirm" "--help"]
        plan.files = "*.yaml"
        apply.files = "*.yaml"
        apply.flags.confirm.values = ["10s" "30s" "60s" "120s"]
    }

    vm = {
        commands = ["list" "attach"]
        flags = ["--vm" "--net" "--help"]
        attach.required = ["--vm" "--net"]
    }

    forward = {
        commands = ["add" "remove" "list"]
        flags = ["--net" "--public" "--dst" "--help"]
        add.required = ["--net" "--public" "--dst"]
        remove.required = ["--net" "--public"]
    }

    policy = {
        commands = ["set" "list"]
        flags = ["--net" "--profile" "--help"]
        set.required = ["--net" "--profile"]
        # Dynamically load profiles from /etc/ghostwarden/policies/
        set.flags.profile.path = "/etc/ghostwarden/policies/*.yaml"
    }

    metrics = {
        commands = ["serve"]
        flags = ["--addr" "--help"]
        serve.flags.addr.values = [":9138" "0.0.0.0:9138" "127.0.0.1:9138"]
    }

    doctor = {
        commands = ["nftables" "docker" "bridges"]
        flags = ["--help"]
    }

    graph = {
        flags = ["--mermaid" "--help"]
        files = "*.yaml"
    }

    tui = {
        flags = ["--help"]
    }
}

# Dynamic completion functions for context-aware suggestions

# Complete network names from topology file
fn complete_network_names {
    if test -f /etc/ghostwarden/ghostnet.yaml {
        grep "^  [a-zA-Z_]" /etc/ghostwarden/ghostnet.yaml | awk '{print $1}' | sed 's/:$//'
    }
}

# Complete policy profiles
fn complete_policy_profiles {
    if test -d /etc/ghostwarden/policies {
        ls /etc/ghostwarden/policies/*.yaml | xargs -n1 basename -s .yaml
    }
}

# Complete libvirt VMs
fn complete_vms {
    if command -v virsh >/dev/null 2>&1 {
        virsh list --all --name 2>/dev/null
    }
}

# Bind dynamic completions to specific contexts
bind-completion gwarden.forward.net {
    source = complete_network_names
}

bind-completion gwarden.policy.net {
    source = complete_network_names
}

bind-completion gwarden.policy.profile {
    source = complete_policy_profiles
}

bind-completion gwarden.vm.vm {
    source = complete_vms
}

# Custom validators for arguments
validate gwarden.forward.public {
    # Validate port format: :PORT/PROTO or IP:PORT/PROTO
    pattern = "^(([0-9]{1,3}\.){3}[0-9]{1,3})?:[0-9]{1,5}/(tcp|udp|sctp)$"
    error = "Format must be :PORT/PROTO or IP:PORT/PROTO"
}

validate gwarden.forward.dst {
    # Validate destination: IP:PORT
    pattern = "^([0-9]{1,3}\.){3}[0-9]{1,3}:[0-9]{1,5}$"
    error = "Format must be IP:PORT"
}

# Contextual help hints
hint gwarden.net.apply.confirm {
    message = "Rollback timeout in seconds (e.g., 30s)"
    examples = ["10s" "30s" "60s" "120s"]
}

hint gwarden.forward.public {
    message = "Public address to forward (e.g., :8080/tcp or 0.0.0.0:8080/tcp)"
    examples = [":8080/tcp" ":443/tcp" "0.0.0.0:22/tcp"]
}

hint gwarden.forward.dst {
    message = "Destination address (e.g., 10.0.0.10:80)"
    examples = ["10.0.0.10:80" "192.168.1.100:22"]
}

# Aliases for common operations
alias gw-plan = "gwarden net plan"
alias gw-apply = "gwarden net apply --commit --confirm 30s"
alias gw-status = "gwarden net status"
alias gw-rollback = "gwarden net rollback"
alias gw-doctor = "gwarden doctor"
alias gw-tui = "gwarden tui"

# Command suggestions based on exit codes
suggest-on-error gwarden {
    # If apply fails, suggest doctor
    1 {
        message = "Configuration apply failed. Try 'gwarden doctor' to diagnose issues."
        commands = ["gwarden doctor" "gwarden net plan"]
    }

    # If validation fails
    2 {
        message = "Topology validation failed. Check your YAML syntax."
        commands = ["gwarden net plan" "gwarden --help"]
    }

    # If permission denied
    13 {
        message = "Permission denied. GhostWarden requires root/CAP_NET_ADMIN."
        commands = ["sudo gwarden net apply --commit --confirm 30s"]
    }
}

# Smart defaults for missing arguments
default gwarden.net.plan {
    file = "/etc/ghostwarden/ghostnet.yaml"
}

default gwarden.net.apply {
    file = "/etc/ghostwarden/ghostnet.yaml"
}

default gwarden.graph {
    file = "/etc/ghostwarden/ghostnet.yaml"
}

# Environment-aware completions
if env GHOSTWARDEN_CONFIG {
    default gwarden.net.plan {
        file = "$GHOSTWARDEN_CONFIG"
    }
    default gwarden.net.apply {
        file = "$GHOSTWARDEN_CONFIG"
    }
}

# Quick commands with keyboard shortcuts
bind ctrl-g-p = "gwarden net plan"
bind ctrl-g-a = "gwarden net apply --commit --confirm 30s"
bind ctrl-g-s = "gwarden net status"
bind ctrl-g-d = "gwarden doctor"
bind ctrl-g-t = "gwarden tui"
