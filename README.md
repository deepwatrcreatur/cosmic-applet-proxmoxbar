# cosmic-applet-proxmoxbar

Native COSMIC applet for monitoring a Proxmox cluster from the panel.

## Status

This is an MVP:

- polls the Proxmox API with token auth
- shows a compact panel summary
- shows cluster, node, guest, and storage details in the popup

## Config

Create `~/.config/cosmic-applet-proxmoxbar/config.toml`:

```toml
base_url = "https://proxmox.example.com:8006"
api_token_id = "root@pam!cosmic-applet-proxmoxbar"
api_token_secret = "replace-me"
verify_tls = true
poll_seconds = 30
```

You can override the config path with `PROXMOXBAR_CONFIG`.

## Run

```bash
cargo run
```

## Package

The repo includes a `flake.nix` that exposes:

- `packages.x86_64-linux.default`
- `apps.x86_64-linux.default`

## Notes

The applet currently uses the standard Proxmox REST API:

- `/api2/json/cluster/status`
- `/api2/json/cluster/resources`
