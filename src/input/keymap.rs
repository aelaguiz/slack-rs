use anyhow::Context as _;
use crossterm::event::KeyEvent;
use keybinds::Keybinds;

use crate::app::action::Action;
use crate::app::config::AppConfig;
use crate::input::normalize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeymapMode {
    Normal,
    Compose,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusArea {
    Sidebar,
    Workspace,
}

pub struct KeyDispatcher {
    normal_sidebar: Keybinds<Action>,
    normal_workspace: Keybinds<Action>,
    compose: Keybinds<Action>,
}

impl KeyDispatcher {
    pub fn new_default() -> anyhow::Result<Self> {
        Self::new(&AppConfig::default())
    }

    pub fn new(config: &AppConfig) -> anyhow::Result<Self> {
        let normal_sidebar = build_normal_sidebar_keybinds(config)?;
        let normal_workspace = build_normal_workspace_keybinds(config)?;
        let compose = build_compose_keybinds(config)?;
        Ok(Self {
            normal_sidebar,
            normal_workspace,
            compose,
        })
    }

    pub fn dispatch(
        &mut self,
        mode: KeymapMode,
        focus: FocusArea,
        key: KeyEvent,
    ) -> Option<Action> {
        let input = normalize::to_key_input(key);
        match mode {
            KeymapMode::Normal => match focus {
                FocusArea::Sidebar => self.normal_sidebar.dispatch(input).cloned(),
                FocusArea::Workspace => self.normal_workspace.dispatch(input).cloned(),
            },
            KeymapMode::Compose => self.compose.dispatch(input).cloned(),
        }
    }
}

fn build_normal_sidebar_keybinds(config: &AppConfig) -> anyhow::Result<Keybinds<Action>> {
    let mut keybinds = Keybinds::default();
    let mut seen = std::collections::BTreeMap::<String, Action>::new();

    // Emergency quit is always available (even if config removes `quit` binds).
    bind_unique(
        &mut keybinds,
        &mut seen,
        "Ctrl+c",
        Action::Quit,
        "built-in emergency quit",
    )?;

    let kb = &config.keybinds;

    // Note: `Ctrl+c` is always bound to quit; default quit is just `q`.
    bind_all(
        &mut keybinds,
        &mut seen,
        kb.quit.as_ref(),
        &["q"],
        Action::Quit,
        "config.keybinds.quit",
    )?;

    bind_all(
        &mut keybinds,
        &mut seen,
        kb.sidebar_refresh.as_ref(),
        &["r"],
        Action::SidebarRefresh,
        "config.keybinds.sidebar_refresh",
    )?;
    bind_all(
        &mut keybinds,
        &mut seen,
        kb.toggle_focus_area.as_ref(),
        &["Tab"],
        Action::ToggleFocusArea,
        "config.keybinds.toggle_focus_area",
    )?;
    bind_all(
        &mut keybinds,
        &mut seen,
        kb.sidebar_up.as_ref(),
        &["k"],
        Action::SidebarSelectPrev,
        "config.keybinds.sidebar_up",
    )?;
    bind_all(
        &mut keybinds,
        &mut seen,
        kb.sidebar_down.as_ref(),
        &["j"],
        Action::SidebarSelectNext,
        "config.keybinds.sidebar_down",
    )?;
    bind_all(
        &mut keybinds,
        &mut seen,
        kb.sidebar_open.as_ref(),
        &["Enter"],
        Action::SidebarOpenSelected,
        "config.keybinds.sidebar_open",
    )?;

    // Note: timeline navigation is bound in the "workspace" keymap; sidebar focus shouldn't
    // accidentally scroll timelines.

    bind_all(
        &mut keybinds,
        &mut seen,
        kb.enter_compose.as_ref(),
        &["i"],
        Action::EnterCompose,
        "config.keybinds.enter_compose",
    )?;

    bind_all(
        &mut keybinds,
        &mut seen,
        kb.file_upload.as_ref(),
        &["f u"],
        Action::FileUploadPromptOpen,
        "config.keybinds.file_upload",
    )?;

    bind_all(
        &mut keybinds,
        &mut seen,
        kb.split_vertical.as_ref(),
        &["Ctrl+w v"],
        Action::SplitVertical,
        "config.keybinds.split_vertical",
    )?;
    bind_all(
        &mut keybinds,
        &mut seen,
        kb.split_horizontal.as_ref(),
        &["Ctrl+w s"],
        Action::SplitHorizontal,
        "config.keybinds.split_horizontal",
    )?;
    bind_all(
        &mut keybinds,
        &mut seen,
        kb.close_pane.as_ref(),
        &["Ctrl+w q"],
        Action::ClosePane,
        "config.keybinds.close_pane",
    )?;

    bind_all(
        &mut keybinds,
        &mut seen,
        kb.focus_left.as_ref(),
        &["Ctrl+w h"],
        Action::FocusLeft,
        "config.keybinds.focus_left",
    )?;
    bind_all(
        &mut keybinds,
        &mut seen,
        kb.focus_down.as_ref(),
        &["Ctrl+w j"],
        Action::FocusDown,
        "config.keybinds.focus_down",
    )?;
    bind_all(
        &mut keybinds,
        &mut seen,
        kb.focus_up.as_ref(),
        &["Ctrl+w k"],
        Action::FocusUp,
        "config.keybinds.focus_up",
    )?;
    bind_all(
        &mut keybinds,
        &mut seen,
        kb.focus_right.as_ref(),
        &["Ctrl+w l"],
        Action::FocusRight,
        "config.keybinds.focus_right",
    )?;

    Ok(keybinds)
}

fn build_normal_workspace_keybinds(config: &AppConfig) -> anyhow::Result<Keybinds<Action>> {
    let mut keybinds = Keybinds::default();
    let mut seen = std::collections::BTreeMap::<String, Action>::new();

    // Emergency quit is always available (even if config removes `quit` binds).
    bind_unique(
        &mut keybinds,
        &mut seen,
        "Ctrl+c",
        Action::Quit,
        "built-in emergency quit",
    )?;

    let kb = &config.keybinds;

    bind_all(
        &mut keybinds,
        &mut seen,
        kb.toggle_focus_area.as_ref(),
        &["Tab"],
        Action::ToggleFocusArea,
        "config.keybinds.toggle_focus_area",
    )?;

    // Note: `Ctrl+c` is always bound to quit; default quit is just `q`.
    bind_all(
        &mut keybinds,
        &mut seen,
        kb.quit.as_ref(),
        &["q"],
        Action::Quit,
        "config.keybinds.quit",
    )?;

    bind_all(
        &mut keybinds,
        &mut seen,
        kb.timeline_up.as_ref(),
        &["k"],
        Action::TimelineSelectPrev,
        "config.keybinds.timeline_up",
    )?;
    bind_all(
        &mut keybinds,
        &mut seen,
        kb.timeline_down.as_ref(),
        &["j"],
        Action::TimelineSelectNext,
        "config.keybinds.timeline_down",
    )?;
    bind_all(
        &mut keybinds,
        &mut seen,
        kb.timeline_top.as_ref(),
        &["g g"],
        Action::TimelineSelectFirst,
        "config.keybinds.timeline_top",
    )?;
    bind_all(
        &mut keybinds,
        &mut seen,
        kb.timeline_bottom.as_ref(),
        &["G"],
        Action::TimelineSelectLast,
        "config.keybinds.timeline_bottom",
    )?;

    bind_all(
        &mut keybinds,
        &mut seen,
        kb.open_thread.as_ref(),
        &["Enter"],
        Action::OpenThreadFromSelection,
        "config.keybinds.open_thread",
    )?;

    bind_all(
        &mut keybinds,
        &mut seen,
        kb.reaction.as_ref(),
        &["r"],
        Action::ReactionPromptOpen,
        "config.keybinds.reaction",
    )?;

    bind_all(
        &mut keybinds,
        &mut seen,
        kb.file_upload.as_ref(),
        &["f u"],
        Action::FileUploadPromptOpen,
        "config.keybinds.file_upload",
    )?;

    bind_all(
        &mut keybinds,
        &mut seen,
        kb.file_download.as_ref(),
        &["f d"],
        Action::FileDownloadPromptOpen,
        "config.keybinds.file_download",
    )?;

    bind_all(
        &mut keybinds,
        &mut seen,
        kb.timeline_load_older.as_ref(),
        &["Ctrl+u"],
        Action::TimelineLoadOlder,
        "config.keybinds.timeline_load_older",
    )?;

    bind_all(
        &mut keybinds,
        &mut seen,
        kb.enter_compose.as_ref(),
        &["i"],
        Action::EnterCompose,
        "config.keybinds.enter_compose",
    )?;

    bind_all(
        &mut keybinds,
        &mut seen,
        kb.split_vertical.as_ref(),
        &["Ctrl+w v"],
        Action::SplitVertical,
        "config.keybinds.split_vertical",
    )?;
    bind_all(
        &mut keybinds,
        &mut seen,
        kb.split_horizontal.as_ref(),
        &["Ctrl+w s"],
        Action::SplitHorizontal,
        "config.keybinds.split_horizontal",
    )?;
    bind_all(
        &mut keybinds,
        &mut seen,
        kb.close_pane.as_ref(),
        &["Ctrl+w q"],
        Action::ClosePane,
        "config.keybinds.close_pane",
    )?;

    bind_all(
        &mut keybinds,
        &mut seen,
        kb.focus_left.as_ref(),
        &["Ctrl+w h"],
        Action::FocusLeft,
        "config.keybinds.focus_left",
    )?;
    bind_all(
        &mut keybinds,
        &mut seen,
        kb.focus_down.as_ref(),
        &["Ctrl+w j"],
        Action::FocusDown,
        "config.keybinds.focus_down",
    )?;
    bind_all(
        &mut keybinds,
        &mut seen,
        kb.focus_up.as_ref(),
        &["Ctrl+w k"],
        Action::FocusUp,
        "config.keybinds.focus_up",
    )?;
    bind_all(
        &mut keybinds,
        &mut seen,
        kb.focus_right.as_ref(),
        &["Ctrl+w l"],
        Action::FocusRight,
        "config.keybinds.focus_right",
    )?;

    Ok(keybinds)
}

fn build_compose_keybinds(config: &AppConfig) -> anyhow::Result<Keybinds<Action>> {
    let mut keybinds = Keybinds::default();
    let mut seen = std::collections::BTreeMap::<String, Action>::new();

    let kb = &config.keybinds;

    bind_unique(
        &mut keybinds,
        &mut seen,
        "Ctrl+c",
        Action::Quit,
        "built-in emergency quit",
    )?;
    bind_unique(
        &mut keybinds,
        &mut seen,
        "Esc",
        Action::LeaveCompose,
        "built-in leave compose",
    )?;

    bind_all(
        &mut keybinds,
        &mut seen,
        kb.composer_send.as_ref(),
        &["Enter"],
        Action::ComposerSend,
        "config.keybinds.composer_send",
    )?;
    bind_all(
        &mut keybinds,
        &mut seen,
        kb.composer_newline.as_ref(),
        &["Shift+Enter"],
        Action::ComposerNewline,
        "config.keybinds.composer_newline",
    )?;

    Ok(keybinds)
}

fn bind_all(
    keybinds: &mut Keybinds<Action>,
    seen: &mut std::collections::BTreeMap<String, Action>,
    configured: Option<&Vec<String>>,
    defaults: &[&str],
    action: Action,
    source: &'static str,
) -> anyhow::Result<()> {
    if let Some(list) = configured {
        for seq in list {
            bind_unique(keybinds, seen, seq, action.clone(), source)?;
        }
        return Ok(());
    }

    for seq in defaults {
        bind_unique(keybinds, seen, seq, action.clone(), source)?;
    }

    Ok(())
}

fn bind_unique(
    keybinds: &mut Keybinds<Action>,
    seen: &mut std::collections::BTreeMap<String, Action>,
    seq: &str,
    action: Action,
    source: &'static str,
) -> anyhow::Result<()> {
    let normalized = normalize_seq(seq);
    if normalized.is_empty() {
        anyhow::bail!("{source} contains an empty key sequence");
    }

    if let Some(existing) = seen.get(&normalized) {
        if *existing != action {
            anyhow::bail!(
                "keybind conflict: '{normalized}' maps to both {existing:?} and {action:?} ({source})"
            );
        }

        // Duplicate binds to the same action are fine, but there's no reason to apply them twice.
        return Ok(());
    }

    keybinds
        .bind(&normalized, action.clone())
        .with_context(|| format!("bind '{normalized}' -> {action:?} ({source})"))?;
    seen.insert(normalized, action);

    Ok(())
}

fn normalize_seq(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}
