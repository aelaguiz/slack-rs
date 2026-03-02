use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders};
use ratatui::Frame;

use crate::app::state::AppState;
use crate::input::keymap::KeymapMode;

pub fn draw_composer(frame: &mut Frame, area: Rect, state: &AppState) {
    let title = match state.mode {
        KeymapMode::Compose => {
            if state.shift_enter_confirmed {
                "Compose  (Enter=send, Shift+Enter=newline, Esc=normal)"
            } else {
                "Compose  (press Shift+Enter once to confirm newline; Esc=normal)"
            }
        }
        KeymapMode::Normal => "Compose  (press i to edit)",
    };

    let block = Block::default().title(title).borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(&state.composer, inner);
}
