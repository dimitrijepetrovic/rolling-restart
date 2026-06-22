# rolling-reboot

A TUI-based tool for performing rolling reboots of servers over SSH. Displays live status of each server in a terminal UI, with servers sorted by status (failed, rebooting, pending) and name.

## Building

```bash
cargo build --release
```

The binary will be at `target/release/rolling-reboot`.

## Usage

```
rolling-reboot [OPTIONS]
```

### Options

| Flag | Description | Default |
|------|-------------|---------|
| `-s, --servers <LIST>` | Comma-separated list of servers (`host` or `host:port`) | — |
| `-f, --file <PATH>` | File containing servers, one per line | — |
| `-c, --concurrency <N>` | Max number of simultaneous reboots | `1` |
| `-t, --timeout <MINUTES>` | Per-server reboot timeout | `20` |
| `--on-timeout <ACTION>` | `stop` the rolling reboot or `continue` on timeout | `stop` |
| `--verify-command <CMD>` | Custom command to verify a server is back up | `ssh <host> uptime` |
| `-u, --user <USER>` | SSH username | `root` |
| `-p, --port <PORT>` | Default SSH port for servers without an explicit port | `22` |

### Server input

Servers can be provided in two ways (or both):

**Inline (comma-separated):**
```bash
rolling-reboot -s web01,web02,web03
```

**From a file** (one server per line, `host` or `host:port`, `#` comments allowed):
```
# Production web servers
web01
web02
web03:2222
db01:2200
```

```bash
rolling-reboot -f servers.txt
```

Both can be combined:
```bash
rolling-reboot -s db01,db02 -f web-servers.txt
```

## Examples

### Reboot 3 servers one at a time
```bash
rolling-reboot -s web01,web02,web03
```

### Reboot with concurrency of 5
```bash
rolling-reboot -f servers.txt -c 5
```

### Use a different SSH user and port
```bash
rolling-reboot -s web01,web02 -u admin -p 2222
```

### Continue rebooting even if a server times out
```bash
rolling-reboot -f servers.txt --on-timeout continue
```

### Set a 10-minute timeout per server
```bash
rolling-reboot -f servers.txt -t 10
```

### Custom verify command
Use `{host}`, `{port}`, and `{user}` as placeholders in the command:

```bash
rolling-reboot -f servers.txt --verify-command "curl -sf http://{host}:8080/health"
```

```bash
rolling-reboot -f servers.txt --verify-command "ssh -p {port} -l {user} {host} 'systemctl is-active nginx'"
```

## TUI

During operation, the terminal displays:

- **Progress bar** at the top showing overall completion, active reboots, failures, and concurrency setting
- **Server table** sorted by status (failed first, then rebooting/verifying, then successful, then pending), then by server name
- **Status bar** at the bottom with current state and instructions

Press `q` at any time to exit.
