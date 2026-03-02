use std::fs;
use std::path::PathBuf;

use anyhow::Context as _;
use directories::ProjectDirs;
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub keybinds: KeybindsConfig,
}

#[derive(Debug, Default, Deserialize)]
pub struct KeybindsConfig {
    pub quit: Option<Vec<String>>,

    pub toggle_focus_area: Option<Vec<String>>,

    pub sidebar_refresh: Option<Vec<String>>,
    pub sidebar_up: Option<Vec<String>>,
    pub sidebar_down: Option<Vec<String>>,
    pub sidebar_open: Option<Vec<String>>,

    pub timeline_load_older: Option<Vec<String>>,
    pub timeline_up: Option<Vec<String>>,
    pub timeline_down: Option<Vec<String>>,
    pub timeline_top: Option<Vec<String>>,
    pub timeline_bottom: Option<Vec<String>>,
    pub open_thread: Option<Vec<String>>,
    pub reaction: Option<Vec<String>>,
    pub file_upload: Option<Vec<String>>,
    pub file_download: Option<Vec<String>>,

    pub enter_compose: Option<Vec<String>>,
    pub composer_send: Option<Vec<String>>,
    pub composer_newline: Option<Vec<String>>,

    pub split_vertical: Option<Vec<String>>,
    pub split_horizontal: Option<Vec<String>>,
    pub close_pane: Option<Vec<String>>,

    pub resize_vertical_plus: Option<Vec<String>>,
    pub resize_vertical_minus: Option<Vec<String>>,
    pub resize_horizontal_plus: Option<Vec<String>>,
    pub resize_horizontal_minus: Option<Vec<String>>,

    pub focus_left: Option<Vec<String>>,
    pub focus_down: Option<Vec<String>>,
    pub focus_up: Option<Vec<String>>,
    pub focus_right: Option<Vec<String>>,
}

#[derive(Debug)]
pub struct LoadedConfig {
    pub config: AppConfig,
    pub path: Option<PathBuf>,
}

pub fn load() -> anyhow::Result<LoadedConfig> {
    for path in candidate_paths() {
        if !path.exists() {
            continue;
        }

        let raw = fs::read_to_string(&path)
            .with_context(|| format!("read config file at {}", path.display()))?;
        let config: AppConfig = toml::from_str(&raw)
            .with_context(|| format!("parse config TOML at {}", path.display()))?;

        return Ok(LoadedConfig {
            config,
            path: Some(path),
        });
    }

    Ok(LoadedConfig {
        config: AppConfig::default(),
        path: None,
    })
}

fn candidate_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();

    // Local-first (for development). This path is ignored by git.
    out.push(PathBuf::from("config.toml"));

    if let Some(xdg) = xdg_config_path() {
        out.push(xdg);
    }

    out
}

fn xdg_config_path() -> Option<PathBuf> {
    // NOTE: strings here only influence filesystem paths.
    ProjectDirs::from("com", "aelaguiz", "slack-rs")
        .map(|dirs| dirs.config_dir().join("config.toml"))
}
