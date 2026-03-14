use anyhow::{Context, Result, anyhow};
use cosmic::{
    Element, Task, app,
    app::Core,
    applet::padded_control,
    iced::{
        Alignment, Length, Subscription,
        futures::{SinkExt, channel::mpsc},
        platform_specific::shell::wayland::commands::popup::{destroy_popup, get_popup},
        widget::{column, row},
        window::Id,
    },
    iced_futures::stream,
    theme,
    widget::{button, container, divider, icon, text},
};
use reqwest::Client;
use serde::Deserialize;
use std::{fs, path::PathBuf, time::Duration};

const APP_ID: &str = "com.deepwatrcreatur.CosmicAppletProxmoxbar";
const CONFIG_ENV: &str = "PROXMOXBAR_CONFIG";
const DEFAULT_POLL_SECONDS: u64 = 30;

pub struct ProxmoxApplet {
    core: Core,
    popup: Option<Id>,
    state: AppState,
}

#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,
    PopupClosed(Id),
    Refresh,
    Refreshed(Result<ClusterSnapshot, String>),
}

#[derive(Debug, Clone)]
struct AppState {
    snapshot: Option<ClusterSnapshot>,
    error: Option<String>,
    status_text: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            snapshot: None,
            error: None,
            status_text: "PVE ...".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct Config {
    base_url: String,
    api_token_id: String,
    api_token_secret: String,
    #[serde(default = "default_verify_tls")]
    verify_tls: bool,
    #[serde(default = "default_poll_seconds")]
    poll_seconds: u64,
}

#[derive(Debug, Clone)]
pub struct ClusterSnapshot {
    cluster_name: Option<String>,
    quorum: bool,
    nodes_online: usize,
    nodes_total: usize,
    guests_running: usize,
    guests_total: usize,
    cpu_fraction: Option<f64>,
    memory_fraction: Option<f64>,
    storage_fraction: Option<f64>,
    nodes: Vec<NodeSummary>,
    guests: Vec<GuestSummary>,
    storages: Vec<StorageSummary>,
}

#[derive(Debug, Clone)]
struct NodeSummary {
    name: String,
    online: bool,
    cpu_fraction: Option<f64>,
    memory_fraction: Option<f64>,
}

#[derive(Debug, Clone)]
struct GuestSummary {
    vmid: u64,
    name: String,
    kind: String,
    status: String,
    node: Option<String>,
}

#[derive(Debug, Clone)]
struct StorageSummary {
    name: String,
    node: Option<String>,
    used_fraction: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct ApiEnvelope<T> {
    data: T,
}

#[derive(Debug, Deserialize)]
struct ClusterStatusEntry {
    #[serde(rename = "type")]
    kind: String,
    name: Option<String>,
    quorate: Option<u8>,
}

#[derive(Debug, Deserialize)]
struct ClusterResource {
    #[serde(rename = "type")]
    kind: String,
    id: Option<String>,
    node: Option<String>,
    name: Option<String>,
    status: Option<String>,
    vmid: Option<u64>,
    cpu: Option<f64>,
    mem: Option<u64>,
    maxmem: Option<u64>,
    disk: Option<u64>,
    maxdisk: Option<u64>,
}

pub fn default_verify_tls() -> bool {
    true
}

pub fn default_poll_seconds() -> u64 {
    DEFAULT_POLL_SECONDS
}

impl cosmic::Application for ProxmoxApplet {
    type Executor = cosmic::SingleThreadExecutor;
    type Flags = ();
    type Message = Message;

    const APP_ID: &'static str = APP_ID;

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: Self::Flags) -> (Self, app::Task<Self::Message>) {
        (
            Self {
                core,
                popup: None,
                state: AppState::default(),
            },
            cosmic::task::message(Message::Refresh),
        )
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::run(poll_subscription)
    }

    fn update(&mut self, message: Self::Message) -> app::Task<Self::Message> {
        match message {
            Message::TogglePopup => {
                return if let Some(popup) = self.popup.take() {
                    destroy_popup(popup)
                } else {
                    let id = Id::unique();
                    self.popup = Some(id);
                    let settings = self.core.applet.get_popup_settings(
                        self.core.main_window_id().unwrap(),
                        id,
                        None,
                        None,
                        None,
                    );
                    get_popup(settings)
                };
            }
            Message::PopupClosed(id) => {
                if self.popup == Some(id) {
                    self.popup = None;
                }
            }
            Message::Refresh => {
                return Task::perform(refresh_snapshot(), |result| {
                    cosmic::Action::App(Message::Refreshed(result.map_err(|err| err.to_string())))
                });
            }
            Message::Refreshed(result) => match result {
                Ok(snapshot) => {
                    self.state.status_text = panel_label(&snapshot);
                    self.state.snapshot = Some(snapshot);
                    self.state.error = None;
                }
                Err(err) => {
                    self.state.status_text = "PVE err".to_string();
                    self.state.error = Some(err);
                    self.state.snapshot = None;
                }
            },
        }

        Task::none()
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let icon_name = match (&self.state.snapshot, &self.state.error) {
            (_, Some(_)) => "dialog-error-symbolic",
            (Some(snapshot), None) if snapshot.quorum => "network-workgroup-symbolic",
            (Some(_), None) => "dialog-warning-symbolic",
            _ => "network-server-symbolic",
        };

        let content = row![
            icon::from_name(icon_name).size(16),
            text(self.state.status_text.as_str()),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        // Hover popup is handled by COSMIC via X-CosmicHoverPopup=true in desktop file
        // Click toggles the popup for accessibility
        button::custom(content)
            .padding([0, self.core.applet.suggested_padding(true).0])
            .on_press(Message::TogglePopup)
            .class(cosmic::theme::Button::AppletIcon)
            .into()
    }

    fn view_window(&self, _id: Id) -> Element<'_, Self::Message> {
        let cosmic::cosmic_theme::Spacing {
            space_xxs, space_s, ..
        } = theme::active().cosmic().spacing;

        let header = if let Some(snapshot) = &self.state.snapshot {
            let title = snapshot
                .cluster_name
                .as_deref()
                .unwrap_or("Proxmox Cluster")
                .to_string();
            column![
                text(title).size(18),
                text(format!(
                    "Nodes {}/{}  Guests {}/{}",
                    snapshot.nodes_online,
                    snapshot.nodes_total,
                    snapshot.guests_running,
                    snapshot.guests_total
                ))
                .size(14),
            ]
            .spacing(4)
        } else if let Some(err) = &self.state.error {
            column![
                text("ProxmoxBar").size(18),
                text(err).size(14),
            ]
            .spacing(4)
        } else {
            column![text("ProxmoxBar").size(18), text("Loading...").size(14)].spacing(4)
        };

        let mut content = column![container(header).padding([12, 16])];

        if let Some(snapshot) = &self.state.snapshot {
            content = content.push(padded_control(divider::horizontal::default()).padding([
                space_xxs, space_s,
            ]));
            content = content.push(section_title("Cluster"));
            content = content.push(info_row(
                "Quorum",
                if snapshot.quorum { "healthy" } else { "degraded" }.to_string(),
            ));
            if let Some(cpu) = snapshot.cpu_fraction {
                content = content.push(info_row("CPU", percent(cpu)));
            }
            if let Some(memory) = snapshot.memory_fraction {
                content = content.push(info_row("Memory", percent(memory)));
            }
            if let Some(storage) = snapshot.storage_fraction {
                content = content.push(info_row("Storage", percent(storage)));
            }

            if !snapshot.nodes.is_empty() {
                content = content.push(padded_control(divider::horizontal::default()).padding([
                    space_xxs, space_s,
                ]));
                content = content.push(section_title("Nodes"));
                for node in &snapshot.nodes {
                    let status = if node.online { "online" } else { "offline" };
                    let mut detail = status.to_string();
                    if let Some(cpu) = node.cpu_fraction {
                        detail.push_str(&format!("  cpu {}", percent(cpu)));
                    }
                    if let Some(memory) = node.memory_fraction {
                        detail.push_str(&format!("  mem {}", percent(memory)));
                    }
                    content = content.push(info_row(node.name.as_str(), detail));
                }
            }

            if !snapshot.guests.is_empty() {
                content = content.push(padded_control(divider::horizontal::default()).padding([
                    space_xxs, space_s,
                ]));
                content = content.push(section_title("Guests"));
                for guest in snapshot.guests.iter().take(8) {
                    let node = guest.node.as_deref().unwrap_or("unknown");
                    content = content.push(info_row(
                        format!("{} {}", guest.kind, guest.vmid),
                        format!("{}  {}  {}", guest.name, guest.status, node),
                    ));
                }
                if snapshot.guests.len() > 8 {
                    content = content.push(
                        text(format!("{} more guests...", snapshot.guests.len() - 8)).size(13),
                    );
                }
            }

            if !snapshot.storages.is_empty() {
                content = content.push(padded_control(divider::horizontal::default()).padding([
                    space_xxs, space_s,
                ]));
                content = content.push(section_title("Storage"));
                for storage in &snapshot.storages {
                    let node = storage.node.as_deref().unwrap_or("cluster");
                    let usage = storage
                        .used_fraction
                        .map(percent)
                        .unwrap_or_else(|| "n/a".to_string());
                    content = content.push(info_row(storage.name.as_str(), format!("{usage}  {node}")));
                }
            }
        } else {
            content = content.push(padded_control(divider::horizontal::default()).padding([
                space_xxs, space_s,
            ]));
            content = content.push(
                text("Create ~/.config/cosmic-applet-proxmoxbar/config.toml to begin.").size(14),
            );
        }

        self.core
            .applet
            .popup_container(container(content.padding([8, 0])))
            .into()
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }
}

fn section_title<'a>(title: &'a str) -> Element<'a, Message> {
    text(title).size(15).into()
}

fn info_row<L: Into<String>, R: Into<String>>(label: L, value: R) -> Element<'static, Message> {
    row![
        text(label.into()).width(Length::Fill),
        text(value.into()),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .into()
}

fn percent(value: f64) -> String {
    format!("{:.0}%", value * 100.0)
}

fn poll_subscription() -> impl cosmic::iced::futures::Stream<Item = Message> {
    stream::channel(1, move |mut output: mpsc::Sender<Message>| async move {
        loop {
            let _ = output.send(Message::Refresh).await;
            let period = Duration::from_secs(
                read_config()
                    .map(|config| config.poll_seconds.max(5))
                    .unwrap_or(DEFAULT_POLL_SECONDS),
            );
            tokio::time::sleep(period).await;
        }
    })
}

fn panel_label(snapshot: &ClusterSnapshot) -> String {
    if snapshot.quorum {
        format!("PVE {}/{}", snapshot.guests_running, snapshot.guests_total)
    } else {
        "PVE warn".to_string()
    }
}

fn config_path() -> PathBuf {
    if let Ok(path) = std::env::var(CONFIG_ENV) {
        return PathBuf::from(path);
    }

    let mut path = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let mut home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
            home.push(".config");
            home
        });
    path.push("cosmic-applet-proxmoxbar");
    path.push("config.toml");
    path
}

fn read_config() -> Result<Config> {
    let path = config_path();
    let content = fs::read_to_string(&path)
        .with_context(|| format!("failed to read config at {}", path.display()))?;
    toml::from_str(&content).context("failed to parse config.toml")
}

async fn refresh_snapshot() -> Result<ClusterSnapshot> {
    let config = read_config()?;
    let client = Client::builder()
        .danger_accept_invalid_certs(!config.verify_tls)
        .build()
        .context("failed to build HTTP client")?;

    let status_url = format!(
        "{}/api2/json/cluster/status",
        config.base_url.trim_end_matches('/')
    );
    let resources_url = format!(
        "{}/api2/json/cluster/resources",
        config.base_url.trim_end_matches('/')
    );
    let auth_value = format!("PVEAPIToken={}={}", config.api_token_id, config.api_token_secret);

    let status = client
        .get(status_url)
        .header("Authorization", &auth_value)
        .send()
        .await
        .context("failed to query cluster status")?
        .error_for_status()
        .context("cluster status request failed")?
        .json::<ApiEnvelope<Vec<ClusterStatusEntry>>>()
        .await
        .context("failed to parse cluster status response")?;

    let resources = client
        .get(resources_url)
        .header("Authorization", &auth_value)
        .send()
        .await
        .context("failed to query cluster resources")?
        .error_for_status()
        .context("cluster resources request failed")?
        .json::<ApiEnvelope<Vec<ClusterResource>>>()
        .await
        .context("failed to parse cluster resources response")?;

    build_snapshot(status.data, resources.data)
}

fn build_snapshot(
    status_entries: Vec<ClusterStatusEntry>,
    resources: Vec<ClusterResource>,
) -> Result<ClusterSnapshot> {
    let cluster_status = status_entries
        .iter()
        .find(|entry| entry.kind == "cluster")
        .ok_or_else(|| anyhow!("cluster status was missing"))?;

    let cluster_name = cluster_status.name.clone();
    let quorum = cluster_status.quorate.unwrap_or(0) == 1;

    let mut nodes = Vec::new();
    let mut guests = Vec::new();
    let mut storages = Vec::new();

    let mut nodes_online = 0usize;
    let mut nodes_total = 0usize;
    let mut guests_running = 0usize;
    let mut guests_total = 0usize;

    let mut total_cpu = 0.0f64;
    let mut cpu_samples = 0usize;
    let mut total_mem = 0u64;
    let mut total_max_mem = 0u64;
    let mut total_disk = 0u64;
    let mut total_max_disk = 0u64;

    for resource in resources {
        match resource.kind.as_str() {
            "node" => {
                nodes_total += 1;
                let online = resource.status.as_deref() == Some("online");
                if online {
                    nodes_online += 1;
                }
                if let Some(cpu) = resource.cpu {
                    total_cpu += cpu;
                    cpu_samples += 1;
                }
                total_mem += resource.mem.unwrap_or(0);
                total_max_mem += resource.maxmem.unwrap_or(0);

                nodes.push(NodeSummary {
                    name: resource
                        .node
                        .or(resource.name)
                        .or(resource.id)
                        .unwrap_or_else(|| "unknown".to_string()),
                    online,
                    cpu_fraction: resource.cpu,
                    memory_fraction: ratio(resource.mem, resource.maxmem),
                });
            }
            "qemu" | "lxc" => {
                guests_total += 1;
                if resource.status.as_deref() == Some("running") {
                    guests_running += 1;
                }

                guests.push(GuestSummary {
                    vmid: resource.vmid.unwrap_or_default(),
                    name: resource
                        .name
                        .unwrap_or_else(|| format!("{} {}", resource.kind, resource.vmid.unwrap_or_default())),
                    kind: resource.kind.to_uppercase(),
                    status: resource.status.unwrap_or_else(|| "unknown".to_string()),
                    node: resource.node,
                });
            }
            "storage" => {
                total_disk += resource.disk.unwrap_or(0);
                total_max_disk += resource.maxdisk.unwrap_or(0);
                storages.push(StorageSummary {
                    name: resource
                        .storage_name()
                        .unwrap_or_else(|| "storage".to_string()),
                    node: resource.node,
                    used_fraction: ratio(resource.disk, resource.maxdisk),
                });
            }
            _ => {}
        }
    }

    guests.sort_by(|a, b| a.vmid.cmp(&b.vmid));
    nodes.sort_by(|a, b| a.name.cmp(&b.name));
    storages.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(ClusterSnapshot {
        cluster_name,
        quorum,
        nodes_online,
        nodes_total,
        guests_running,
        guests_total,
        cpu_fraction: if cpu_samples > 0 {
            Some(total_cpu / cpu_samples as f64)
        } else {
            None
        },
        memory_fraction: ratio(Some(total_mem), Some(total_max_mem)),
        storage_fraction: ratio(Some(total_disk), Some(total_max_disk)),
        nodes,
        guests,
        storages,
    })
}

fn ratio<T: Into<u64>>(used: Option<T>, total: Option<T>) -> Option<f64> {
    let used = used?.into();
    let total = total?.into();
    if total == 0 {
        return None;
    }
    Some(used as f64 / total as f64)
}

impl ClusterResource {
    fn storage_name(&self) -> Option<String> {
        self.id
            .as_deref()
            .and_then(|id| id.rsplit('/').next())
            .map(str::to_string)
            .or_else(|| self.name.clone())
    }
}
