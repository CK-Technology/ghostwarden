# Commands

This page is generated from the `gwarden` clap command metadata.

### `gwarden`

```text
Ghost network orchestration

Usage: gwarden <COMMAND>

Commands:
  net      Network management
  vm       VM operations
  forward  Port forwarding
  policy   Policy management
  tui      Terminal UI
  metrics  Metrics server
  doctor   Troubleshooting and diagnostics
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help
          Print help

  -V, --version
          Print version
```

### `gwarden net`

```text
Network management

Usage: net <COMMAND>

Commands:
  plan         Show planned changes without applying
  apply        Apply network configuration
  status       Show current network status
  diff         Compare desired nftables rules with live system
  rollback     Roll back the last applied configuration snapshot
  state        Show the persisted apply state from the last commit
  state-clear  Clear the persisted apply state (does not touch live resources)
  help         Print this message or the help of the given subcommand(s)

Options:
  -h, --help
          Print help
```

### `gwarden net plan`

```text
Show planned changes without applying

Usage: plan [OPTIONS]

Options:
  -f, --file <FILE>
          [default: ghostnet.toml]

  -h, --help
          Print help
```

### `gwarden net apply`

```text
Apply network configuration

Usage: apply [OPTIONS]

Options:
  -f, --file <FILE>
          [default: ghostnet.toml]

      --commit
          Execute changes on the host; without this flag apply is a dry run

      --confirm <CONFIRM>
          Auto-rollback window in seconds; press ENTER to confirm, 0 disables the wait
          
          [default: 30]

      --probe <PROBE>
          host:port to probe for connectivity; rollback runs if it is unreachable

      --probe-timeout <PROBE_TIMEOUT>
          Timeout in seconds for the connectivity probe
          
          [default: 3]

  -h, --help
          Print help
```

### `gwarden net status`

```text
Show current network status

Usage: status

Options:
  -h, --help
          Print help
```

### `gwarden net diff`

```text
Compare desired nftables rules with live system

Usage: diff [OPTIONS]

Options:
  -f, --file <FILE>
          [default: ghostnet.toml]

      --table <TABLE>
          Only diff nftables tables (or networks) matching this name

  -h, --help
          Print help
```

### `gwarden net rollback`

```text
Roll back the last applied configuration snapshot

Usage: rollback [OPTIONS]

Options:
      --execute
          Execute the rollback; without this flag only a preview is printed

  -h, --help
          Print help
```

### `gwarden net state`

```text
Show the persisted apply state from the last commit

Usage: state [OPTIONS]

Options:
      --json
          Emit the apply state as JSON instead of a human summary

  -h, --help
          Print help
```

### `gwarden net state-clear`

```text
Clear the persisted apply state (does not touch live resources)

Usage: state-clear [OPTIONS]

Options:
      --confirm
          Required acknowledgement; clearing state is irreversible

  -h, --help
          Print help
```

### `gwarden vm`

```text
VM operations

Usage: vm <COMMAND>

Commands:
  attach  Attach VM to network
  list    List VMs and their network attachments
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help
          Print help
```

### `gwarden vm attach`

```text
Attach VM to network

Usage: attach [OPTIONS] --vm <VM> --net <NET>

Options:
      --vm <VM>
          

      --net <NET>
          

      --tap <TAP>
          

  -h, --help
          Print help
```

### `gwarden vm list`

```text
List VMs and their network attachments

Usage: list

Options:
  -h, --help
          Print help
```

### `gwarden forward`

```text
Port forwarding

Usage: forward <COMMAND>

Commands:
  add     Add port forward
  remove  Remove port forward
  list    List port forwards
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help
          Print help
```

### `gwarden forward add`

```text
Add port forward

Usage: add --net <NET> --public <PUBLIC> --dst <DST>

Options:
      --net <NET>
          

      --public <PUBLIC>
          

      --dst <DST>
          

  -h, --help
          Print help
```

### `gwarden forward remove`

```text
Remove port forward

Usage: remove --net <NET> --public <PUBLIC>

Options:
      --net <NET>
          

      --public <PUBLIC>
          

  -h, --help
          Print help
```

### `gwarden forward list`

```text
List port forwards

Usage: list

Options:
  -h, --help
          Print help
```

### `gwarden policy`

```text
Policy management

Usage: policy <COMMAND>

Commands:
  set   Set policy profile for network
  list  List available policy profiles
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help
          Print help
```

### `gwarden policy set`

```text
Set policy profile for network

Usage: set [OPTIONS] --net <NET> --profile <PROFILE>

Options:
      --net <NET>
          

      --profile <PROFILE>
          

      --file <FILE>
          [default: ghostnet.toml]

  -h, --help
          Print help
```

### `gwarden policy list`

```text
List available policy profiles

Usage: list

Options:
  -h, --help
          Print help
```

### `gwarden metrics`

```text
Metrics server

Usage: metrics <COMMAND>

Commands:
  serve  Start metrics server
  help   Print this message or the help of the given subcommand(s)

Options:
  -h, --help
          Print help
```

### `gwarden metrics serve`

```text
Start metrics server

Usage: serve [OPTIONS]

Options:
      --addr <ADDR>
          [default: :9138]

  -h, --help
          Print help
```

### `gwarden doctor`

```text
Troubleshooting and diagnostics

Usage: doctor [COMMAND]

Commands:
  nftables  Check nftables/iptables configuration
  docker    Check Docker networking
  bridges   Check bridge configuration
  all       Run all diagnostics
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help
          Print help
```

### `gwarden doctor nftables`

```text
Check nftables/iptables configuration

Usage: nftables

Options:
  -h, --help
          Print help
```

### `gwarden doctor docker`

```text
Check Docker networking

Usage: docker

Options:
  -h, --help
          Print help
```

### `gwarden doctor bridges`

```text
Check bridge configuration

Usage: bridges

Options:
  -h, --help
          Print help
```

### `gwarden doctor all`

```text
Run all diagnostics

Usage: all

Options:
  -h, --help
          Print help
```

### `gwarden tui`

```text
Terminal UI

Usage: tui

Options:
  -h, --help
          Print help
```

