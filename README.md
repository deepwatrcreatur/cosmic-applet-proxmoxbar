# cosmic-applet-proxmoxbar

Native COSMIC desktop applet for monitoring a Proxmox cluster from the panel.

## Features

- Real-time cluster status in the panel (guests running/total)
- Hover popup with detailed information:
  - Cluster quorum status
  - CPU, memory, and storage usage
  - Node status (online/offline)
  - Guest VMs and containers
  - Storage pools
- Configurable polling interval
- API token authentication

## Installation

### Nix Flake

```nix
# In your flake inputs
inputs.cosmic-applet-proxmoxbar.url = "github:deepwatrcreatur/cosmic-applet-proxmoxbar";

# In home.packages or environment.systemPackages
inputs.cosmic-applet-proxmoxbar.packages.${system}.default
```

### From Source

```bash
cargo build --release
cp target/release/cosmic-applet-proxmoxbar ~/.local/bin/
cp data/com.deepwatrcreatur.CosmicAppletProxmoxbar.desktop ~/.local/share/applications/
```

## Configuration

Create `~/.config/cosmic-applet-proxmoxbar/config.toml`:

```toml
base_url = "https://proxmox.example.com:8006"
api_token_id = "root@pam!mytoken"
api_token_secret = "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
verify_tls = false
poll_seconds = 30
```

### Config Options

| Option | Description | Default |
|--------|-------------|---------|
| `base_url` | Proxmox API URL (include port 8006) | required |
| `api_token_id` | API token in format `user@realm!tokenname` | required |
| `api_token_secret` | The token secret UUID | required |
| `verify_tls` | Verify TLS certificates | `true` |
| `poll_seconds` | Refresh interval in seconds (minimum 5) | `30` |

You can override the config path with the `PROXMOXBAR_CONFIG` environment variable.

## Creating a Proxmox API Token

1. SSH to your Proxmox server or use the web UI

2. Create the token:
   ```bash
   pveum user token add root@pam cosmic-applet-proxmoxbar --privsep=0
   ```

3. **Important**: Set `--privsep=0` to disable privilege separation, otherwise the token won't inherit permissions and API calls will fail with 401.

4. Copy the displayed token secret to your config file.

### Via Web UI

1. Go to **Datacenter > Permissions > API Tokens**
2. Click **Add**
3. User: `root@pam` (or your preferred user)
4. Token ID: `cosmic-applet-proxmoxbar`
5. **Uncheck** "Privilege Separation"
6. Copy the secret (only shown once!)

## Adding to COSMIC Panel

1. Open **COSMIC Settings > Desktop > Panel**
2. Click on your panel (top or bottom)
3. Go to **Applets** section
4. Find "ProxmoxBar" and add it
5. The applet will appear in your panel

## Troubleshooting

### "Failed to query cluster" error

1. **Check connectivity**: Can you reach the Proxmox host?
   ```bash
   curl -sk https://proxmox.example.com:8006/api2/json/version
   ```

2. **Verify token**: Test the API token:
   ```bash
   curl -sk https://proxmox.example.com:8006/api2/json/cluster/status \
     -H "Authorization: PVEAPIToken=root@pam!mytoken=your-secret-here"
   ```

3. **TLS issues**: If using self-signed certs, set `verify_tls = false`

4. **Token permissions**: Ensure privilege separation is disabled on the token

### 401 Unauthorized

- The token secret may be incorrect
- Privilege separation may be enabled on the token
- The token may have been deleted/recreated (secrets change)

### Popup won't close

- Click the applet icon again to toggle the popup
- Or hover away and back to refresh

### Applet not appearing in panel

- Ensure the desktop file is installed to `~/.local/share/applications/` or system applications directory
- Log out and back in, or restart cosmic-panel

## API Endpoints Used

The applet queries these Proxmox API endpoints:

- `GET /api2/json/cluster/status` - Cluster and node status
- `GET /api2/json/cluster/resources` - VMs, containers, and storage

## License

MIT
