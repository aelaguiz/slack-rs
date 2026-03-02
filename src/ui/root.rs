use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::prompt::PromptKind;
use crate::app::state::AppState;
use crate::input::keymap::{FocusArea, KeymapMode};

pub fn draw_root(frame: &mut Frame, state: &AppState) {
    let root = frame.area();

    let [status, body, composer] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .areas(root);

    let mode = match state.mode {
        KeymapMode::Normal => "NORMAL",
        KeymapMode::Compose => "COMPOSE",
    };

    let socket_mode = if state.socket_mode_connected {
        format!("connected ({} events)", state.socket_mode_events)
    } else {
        "not connected".to_string()
    };

    let prompt = state.prompt.as_ref().map(|p| {
        let raw = p.value();
        let mut s = raw.trim().to_string();
        if s.len() > 32 {
            s.truncate(32);
            s.push('…');
        }

        let label = match p.kind {
            PromptKind::ReactionEmoji => "react",
            PromptKind::FileUploadPath => "upload",
            PromptKind::FileDownloadPath => "download",
        };

        format!("{label}: {s} (Enter=apply Esc=cancel)")
    });

    let status_text = Paragraph::new(format!(
        "slack-rs  |  SocketMode: {socket_mode}  |  [{mode}]  |  focus: {}{}",
        match state.focus_area {
            FocusArea::Sidebar => "SIDEBAR",
            FocusArea::Workspace => "WORKSPACE",
        },
        prompt
            .as_ref()
            .map(|v| format!("  |  {v}"))
            .unwrap_or_default()
    ))
    .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(status_text, status);

    let [sidebar, workspace] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(28), Constraint::Min(0)])
        .areas(body);

    crate::ui::sidebar::draw_sidebar(frame, sidebar, state);
    crate::ui::workspace::draw_workspace(frame, workspace, state);

    crate::ui::composer::draw_composer(frame, composer, state);
}
