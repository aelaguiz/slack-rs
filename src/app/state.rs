use std::collections::BTreeMap;

use tui_textarea::TextArea;

use crate::input::keymap::{FocusArea, KeymapMode};
use crate::model::{ConversationId, ThreadKey};
use crate::workspace::tree::PaneId;
use crate::workspace::tree::PaneTree;

use crate::app::pane_view::PaneViewState;
use crate::app::prompt::PromptState;
use crate::app::sidebar::SidebarState;
use crate::app::timeline::TimelineState;

#[derive(Debug)]
pub struct AppState {
    pub mode: KeymapMode,
    pub last_key: Option<String>,
    pub last_sent: Option<String>,
    pub last_uploaded_file_id: Option<String>,
    pub last_downloaded_path: Option<String>,
    pub last_downloaded_bytes: Option<u64>,
    pub shift_enter_confirmed: bool,
    pub composer: TextArea<'static>,
    pub prompt: Option<PromptState>,
    pub last_reaction_emoji: String,
    pub workspace: PaneTree,
    pub focus_area: FocusArea,
    pub pane_views: BTreeMap<PaneId, PaneViewState>,

    pub socket_mode_connected: bool,
    pub socket_mode_events: u64,

    pub sidebar: SidebarState,
    pub timelines: BTreeMap<ConversationId, TimelineState>,
    pub threads: BTreeMap<ThreadKey, TimelineState>,
}

impl Default for AppState {
    fn default() -> Self {
        let workspace = PaneTree::default();
        let focused = workspace.focused();

        let mut pane_views = BTreeMap::new();
        pane_views.insert(focused, PaneViewState::default());

        Self {
            mode: KeymapMode::Normal,
            last_key: None,
            last_sent: None,
            last_uploaded_file_id: None,
            last_downloaded_path: None,
            last_downloaded_bytes: None,
            shift_enter_confirmed: false,
            composer: TextArea::default(),
            prompt: None,
            last_reaction_emoji: "eyes".to_string(),
            workspace,
            focus_area: FocusArea::Sidebar,
            pane_views,

            socket_mode_connected: false,
            socket_mode_events: 0,

            sidebar: SidebarState::default(),
            timelines: BTreeMap::new(),
            threads: BTreeMap::new(),
        }
    }
}
