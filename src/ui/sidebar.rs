use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::state::AppState;
use crate::input::keymap::FocusArea;
use crate::model::ThreadSummary;
use crate::model::{ConversationKind, ConversationSummary};

pub fn draw_sidebar(frame: &mut Frame, area: Rect, state: &AppState) {
    let is_focused = state.focus_area == FocusArea::Sidebar;
    let header_style = Style::default()
        .add_modifier(Modifier::BOLD)
        .fg(Color::Cyan);
    let selected_style = if is_focused {
        Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(Color::Black)
            .bg(Color::Yellow)
    } else {
        Style::default()
            .add_modifier(Modifier::BOLD)
            .fg(Color::Yellow)
    };

    let mut text = Text::default();

    push_header(&mut text, "Threads (recent/open)", header_style);
    if state.sidebar.threads.is_empty() {
        push_dim(
            &mut text,
            "  (none yet)",
            Style::default().fg(Color::DarkGray),
        );
    } else {
        for (flat_idx, thread) in state.sidebar.threads.iter().enumerate() {
            let is_selected = state.sidebar.selected_idx == Some(flat_idx);
            push_thread(&mut text, thread, is_selected, selected_style);
        }
    }
    text.lines.push(Line::from(""));

    push_header(&mut text, "Channels", header_style);
    if state.sidebar.channels.is_empty() && !state.sidebar.loading {
        push_dim(&mut text, "  (none)", Style::default().fg(Color::DarkGray));
    } else {
        let offset = state.sidebar.threads.len();
        for (idx, conv) in state.sidebar.channels.iter().enumerate() {
            let flat_idx = offset + idx;
            let is_selected = state.sidebar.selected_idx == Some(flat_idx);
            push_conv(&mut text, conv, is_selected, selected_style);
        }
    }
    text.lines.push(Line::from(""));

    push_header(&mut text, "DMs", header_style);
    if state.sidebar.dms.is_empty() && !state.sidebar.loading {
        push_dim(&mut text, "  (none)", Style::default().fg(Color::DarkGray));
    } else {
        let offset = state.sidebar.threads.len() + state.sidebar.channels.len();
        for (idx, conv) in state.sidebar.dms.iter().enumerate() {
            let flat_idx = offset + idx;
            let is_selected = state.sidebar.selected_idx == Some(flat_idx);
            push_conv(&mut text, conv, is_selected, selected_style);
        }
    }

    let border_type = if is_focused {
        BorderType::Double
    } else {
        BorderType::Plain
    };

    let widget = Paragraph::new(text)
        .block(
            Block::default()
                .title("Slack bar")
                .borders(Borders::ALL)
                .border_type(border_type),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(widget, area);
}

fn push_header(text: &mut Text<'_>, label: &'static str, style: Style) {
    text.lines.push(Line::from(Span::styled(label, style)));
}

fn push_dim(text: &mut Text<'_>, label: &'static str, style: Style) {
    text.lines.push(Line::from(Span::styled(label, style)));
}

fn push_conv(text: &mut Text<'_>, conv: &ConversationSummary, is_selected: bool, selected: Style) {
    let prefix = if is_selected { "> " } else { "  " };
    let mut title = conv.display_title();

    if conv.kind == ConversationKind::PrivateChannel {
        title = format!("🔒{title}");
    }

    if let Some(count) = conv.unread_count_display.filter(|c| *c > 0) {
        title = format!("{title} ({count})");
    }

    let line = format!("{prefix}{title}");
    if is_selected {
        text.lines.push(Line::from(Span::styled(line, selected)));
        return;
    }

    let inaccessible = matches!(
        conv.kind,
        ConversationKind::Channel | ConversationKind::PrivateChannel
    ) && !conv.is_member;

    if inaccessible {
        text.lines.push(Line::from(Span::styled(
            line,
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        text.lines.push(Line::from(line));
    }
}

fn push_thread(text: &mut Text<'_>, thread: &ThreadSummary, is_selected: bool, selected: Style) {
    let prefix = if is_selected { "> " } else { "  " };
    let line = format!("{prefix}🧵 {}", thread.title);
    if is_selected {
        text.lines.push(Line::from(Span::styled(line, selected)));
    } else {
        text.lines.push(Line::from(line));
    }
}
