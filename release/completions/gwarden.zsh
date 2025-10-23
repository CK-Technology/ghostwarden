#compdef gwarden

_gwarden() {
    local -a commands
    commands=(
        'net:Network topology management'
        'vm:Virtual machine network management'
        'forward:Port forwarding management'
        'policy:Security policy management'
        'metrics:Prometheus metrics server'
        'doctor:Network diagnostics'
        'graph:Topology visualization'
        'tui:Terminal UI dashboard'
        'help:Print help information'
    )

    local -a net_cmds
    net_cmds=(
        'plan:Preview network changes'
        'apply:Apply network configuration'
        'status:Show current network status'
        'rollback:Rollback to previous configuration'
    )

    local -a vm_cmds
    vm_cmds=(
        'list:List available VMs'
        'attach:Attach VM to network'
    )

    local -a forward_cmds
    forward_cmds=(
        'add:Add port forward'
        'remove:Remove port forward'
        'list:List port forwards'
    )

    local -a policy_cmds
    policy_cmds=(
        'set:Set policy profile'
        'list:List policy profiles'
    )

    local -a doctor_cmds
    doctor_cmds=(
        'nftables:Check nftables configuration'
        'docker:Check Docker networking'
        'bridges:Check bridge configuration'
    )

    _arguments -C \
        '1: :->cmds' \
        '*:: :->args' \
        && return 0

    case $state in
        cmds)
            _describe 'gwarden command' commands
            ;;
        args)
            case $words[1] in
                net)
                    _arguments -C \
                        '1: :->net_subcmds' \
                        '*:: :->net_args' \
                        && return 0

                    case $state in
                        net_subcmds)
                            _describe 'net subcommand' net_cmds
                            ;;
                        net_args)
                            case $words[1] in
                                plan|apply)
                                    _arguments \
                                        '--commit[Actually apply changes]' \
                                        '--confirm[Rollback timeout]:seconds:(10s 30s 60s 120s)' \
                                        '1:topology file:_files -g "*.yaml"'
                                    ;;
                                *)
                                    _message 'no more arguments'
                                    ;;
                            esac
                            ;;
                    esac
                    ;;
                vm)
                    _arguments -C \
                        '1: :->vm_subcmds' \
                        '*:: :->vm_args' \
                        && return 0

                    case $state in
                        vm_subcmds)
                            _describe 'vm subcommand' vm_cmds
                            ;;
                        vm_args)
                            _arguments \
                                '--vm[VM name]:vm:' \
                                '--net[Network name]:network:'
                            ;;
                    esac
                    ;;
                forward)
                    _arguments -C \
                        '1: :->fwd_subcmds' \
                        '*:: :->fwd_args' \
                        && return 0

                    case $state in
                        fwd_subcmds)
                            _describe 'forward subcommand' forward_cmds
                            ;;
                        fwd_args)
                            _arguments \
                                '--net[Network name]:network:' \
                                '--public[Public address]:public:' \
                                '--dst[Destination]:destination:'
                            ;;
                    esac
                    ;;
                policy)
                    _arguments -C \
                        '1: :->pol_subcmds' \
                        '*:: :->pol_args' \
                        && return 0

                    case $state in
                        pol_subcmds)
                            _describe 'policy subcommand' policy_cmds
                            ;;
                        pol_args)
                            local -a profiles
                            if [[ -d /etc/ghostwarden/policies ]]; then
                                profiles=(${(f)"$(ls /etc/ghostwarden/policies/*.yaml 2>/dev/null | xargs -n1 basename -s .yaml)"})
                            fi
                            _arguments \
                                '--net[Network name]:network:' \
                                '--profile[Policy profile]:profile:($profiles)'
                            ;;
                    esac
                    ;;
                metrics)
                    _arguments \
                        '1:subcommand:(serve)' \
                        '--addr[Bind address]:address:(:9138 0.0.0.0:9138)'
                    ;;
                doctor)
                    _arguments \
                        '1: :->doc_subcmds' \
                        && return 0

                    case $state in
                        doc_subcmds)
                            _describe 'doctor check' doctor_cmds
                            ;;
                    esac
                    ;;
                graph)
                    _arguments \
                        '--mermaid[Output Mermaid diagram]' \
                        '1:topology file:_files -g "*.yaml"'
                    ;;
                tui)
                    _message 'no arguments'
                    ;;
            esac
            ;;
    esac

    return 0
}

_gwarden "$@"
