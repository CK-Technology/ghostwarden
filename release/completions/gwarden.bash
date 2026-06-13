# Bash completion for gwarden

_gwarden() {
    local cur prev opts
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"

    # Top-level commands
    local commands="net vm forward policy metrics doctor tui help"

    # If we're completing the first argument
    if [ $COMP_CWORD -eq 1 ]; then
        COMPREPLY=( $(compgen -W "${commands}" -- ${cur}) )
        return 0
    fi

    # Get the subcommand
    local subcommand="${COMP_WORDS[1]}"

    case "${subcommand}" in
        net)
            local net_cmds="plan apply status diff rollback state state-clear"
            if [ $COMP_CWORD -eq 2 ]; then
                COMPREPLY=( $(compgen -W "${net_cmds}" -- ${cur}) )
            elif [[ "${prev}" == "--confirm" ]]; then
                # Suggest rollback-window values in seconds (0 disables the wait)
                COMPREPLY=( $(compgen -W "0 10 30 60 120" -- ${cur}) )
            elif [[ "${prev}" == "-f" || "${prev}" == "--file" ]]; then
                # Complete with supported topology files for plan/apply/diff
                COMPREPLY=( $(compgen -f -X '!*.toml' -- ${cur}) )
                COMPREPLY+=( $(compgen -f -X '!*.yaml' -- ${cur}) )
                COMPREPLY+=( $(compgen -f -X '!*.yml' -- ${cur}) )
            elif [[ "${cur}" == -* ]]; then
                local opts="--file --commit --confirm --probe --probe-timeout --table --execute --json --help"
                COMPREPLY=( $(compgen -W "${opts}" -- ${cur}) )
            fi
            ;;
        vm)
            local vm_cmds="list attach"
            if [ $COMP_CWORD -eq 2 ]; then
                COMPREPLY=( $(compgen -W "${vm_cmds}" -- ${cur}) )
            elif [[ "${cur}" == -* ]]; then
                local opts="--vm --net --help"
                COMPREPLY=( $(compgen -W "${opts}" -- ${cur}) )
            fi
            ;;
        forward)
            local fwd_cmds="add remove list"
            if [ $COMP_CWORD -eq 2 ]; then
                COMPREPLY=( $(compgen -W "${fwd_cmds}" -- ${cur}) )
            elif [[ "${cur}" == -* ]]; then
                local opts="--net --public --dst --help"
                COMPREPLY=( $(compgen -W "${opts}" -- ${cur}) )
            fi
            ;;
        policy)
            local pol_cmds="set list"
            if [ $COMP_CWORD -eq 2 ]; then
                COMPREPLY=( $(compgen -W "${pol_cmds}" -- ${cur}) )
            elif [[ "${prev}" == "--profile" ]]; then
                # Suggest policy profiles from /etc/gwarden/policies/
                if [ -d /etc/gwarden/policies ]; then
                    local profiles=$(ls /etc/gwarden/policies/*.toml 2>/dev/null | xargs -n1 basename -s .toml)
                    COMPREPLY=( $(compgen -W "${profiles}" -- ${cur}) )
                fi
            elif [[ "${cur}" == -* ]]; then
                local opts="--net --profile --help"
                COMPREPLY=( $(compgen -W "${opts}" -- ${cur}) )
            fi
            ;;
        metrics)
            local met_cmds="serve"
            if [ $COMP_CWORD -eq 2 ]; then
                COMPREPLY=( $(compgen -W "${met_cmds}" -- ${cur}) )
            elif [[ "${prev}" == "--addr" ]]; then
                COMPREPLY=( $(compgen -W ":9138 0.0.0.0:9138 127.0.0.1:9138" -- ${cur}) )
            elif [[ "${cur}" == -* ]]; then
                local opts="--addr --help"
                COMPREPLY=( $(compgen -W "${opts}" -- ${cur}) )
            fi
            ;;
        doctor)
            local doc_cmds="nftables docker bridges all"
            if [ $COMP_CWORD -eq 2 ]; then
                COMPREPLY=( $(compgen -W "${doc_cmds}" -- ${cur}) )
            elif [[ "${cur}" == -* ]]; then
                COMPREPLY=( $(compgen -W "--help" -- ${cur}) )
            fi
            ;;
        tui)
            if [[ "${cur}" == -* ]]; then
                COMPREPLY=( $(compgen -W "--help" -- ${cur}) )
            fi
            ;;
    esac

    return 0
}

complete -F _gwarden gwarden
