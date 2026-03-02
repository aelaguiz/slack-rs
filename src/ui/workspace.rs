use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::app::state::AppState;
use crate::input::keymap::FocusArea;
use crate::model::{ConversationId, ThreadKey};
use crate::workspace::render;
use crate::workspace::tree::PaneKind;

pub fn draw_workspace(frame: &mut Frame, area: Rect, state: &AppState) {
    let focused = state.workspace.focused();
    let layouts = render::layout(&state.workspace, area);

    for pane in layouts {
        let is_focused = pane.id == focused;
        let is_active = is_focused && state.focus_area == FocusArea::Workspace;
        let border_type = if is_active {
            BorderType::Double
        } else {
            BorderType::Plain
        };

        let title = if is_focused {
            format!("Pane {} (focused)", pane.id.get())
        } else {
            format!("Pane {}", pane.id.get())
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(border_type)
            .style(Style::default().fg(if is_focused {
                Color::Yellow
            } else {
                Color::Reset
            }));

        match &pane.kind {
            PaneKind::Placeholder => {
                let widget = Paragraph::new("(empty)").block(block);
                frame.render_widget(widget, pane.area);
            }
            PaneKind::Timeline { conversation } => {
                frame.render_widget(block.clone(), pane.area);
                let inner = block.inner(pane.area);
                if inner.height == 0 || inner.width == 0 {
                    continue;
                }

                let title = conversation_title(state, conversation);

                let Some(timeline) = state.timelines.get(conversation) else {
                    let widget = Paragraph::new(format!("Timeline: {title}\n\nloading…"));
                    frame.render_widget(widget, inner);
                    continue;
                };
                if timeline.loading {
                    let widget = Paragraph::new(format!("Timeline: {title}\n\nloading…"));
                    frame.render_widget(widget, inner);
                    continue;
                }
                if timeline.messages.is_empty() {
                    let widget =
                        Paragraph::new(format!("Timeline: {title}\n\n(no messages loaded yet)"));
                    frame.render_widget(widget, inner);
                    continue;
                }

                // Display order: oldest → newest.
                let display = timeline.messages.iter().rev().collect::<Vec<_>>();
                let mut items = Vec::<ListItem>::with_capacity(display.len());
                let mut selected_idx = None;

                let selected_ts = state
                    .pane_views
                    .get(&pane.id)
                    .and_then(|v| v.selected_ts.as_ref());

                for (idx, msg) in display.iter().enumerate() {
                    if selected_idx.is_none()
                        && selected_ts.is_some_and(|ts| ts.as_str() == msg.ts.as_str())
                    {
                        selected_idx = Some(idx);
                    }

                    let who = msg.user.as_ref().map(|u| u.as_str()).unwrap_or("<unknown>");
                    let mut header = format!("{who} {}", msg.ts.as_str());
                    if msg.thread_ts.is_some() {
                        header.push_str(" (thread)");
                    }
                    if !msg.reactions.is_empty() {
                        header.push_str("  ");
                        header.push_str(&format_reactions(&msg.reactions));
                    }

                    let mut lines = Vec::<Line>::new();
                    lines.push(Line::from(Span::styled(
                        header,
                        Style::default().add_modifier(Modifier::BOLD),
                    )));

                    let rendered = crate::render::mrkdwn::render_mrkdwn_to_plaintext(&msg.text);
                    for line in rendered.lines() {
                        if line.trim().is_empty() {
                            lines.push(Line::from(""));
                        } else {
                            lines.push(Line::from(Span::raw(format!("  {}", line.trim_end()))));
                        }
                    }

                    for line in crate::render::attachments::render_files_to_plaintext(&msg.files) {
                        lines.push(Line::from(Span::raw(format!("  {}", line.trim_end()))));
                    }

                    // Spacer between messages.
                    lines.push(Line::from(""));

                    items.push(ListItem::new(Text::from(lines)));
                }

                if is_active && selected_idx.is_none() {
                    selected_idx = Some(display.len().saturating_sub(1));
                }

                let highlight_style = Style::default()
                    .add_modifier(Modifier::REVERSED)
                    .fg(Color::Black)
                    .bg(Color::Yellow);

                let list = List::new(items)
                    .highlight_style(highlight_style)
                    .highlight_symbol(">> ");

                let mut list_state =
                    ListState::default().with_selected(if is_active { selected_idx } else { None });

                frame.render_stateful_widget(list, inner, &mut list_state);
            }
            PaneKind::Thread {
                conversation,
                thread_ts,
            } => {
                frame.render_widget(block.clone(), pane.area);
                let inner = block.inner(pane.area);
                if inner.height == 0 || inner.width == 0 {
                    continue;
                }

                let conv_title = conversation_title(state, conversation);
                let title = format!("Thread: {conv_title} @ {thread_ts}");
                let key = ThreadKey {
                    conversation: conversation.clone(),
                    thread_ts: thread_ts.clone(),
                };

                let Some(thread) = state.threads.get(&key) else {
                    let widget = Paragraph::new(format!("{title}\n\nloading…"));
                    frame.render_widget(widget, inner);
                    continue;
                };
                if thread.loading {
                    let widget = Paragraph::new(format!("{title}\n\nloading…"));
                    frame.render_widget(widget, inner);
                    continue;
                }
                if thread.messages.is_empty() {
                    let widget = Paragraph::new(format!("{title}\n\n(no replies loaded yet)"));
                    frame.render_widget(widget, inner);
                    continue;
                }

                // Display order: oldest → newest.
                let display = thread.messages.iter().rev().collect::<Vec<_>>();
                let mut items = Vec::<ListItem>::with_capacity(display.len());
                let mut selected_idx = None;

                let selected_ts = state
                    .pane_views
                    .get(&pane.id)
                    .and_then(|v| v.selected_ts.as_ref());

                for (idx, msg) in display.iter().enumerate() {
                    if selected_idx.is_none()
                        && selected_ts.is_some_and(|ts| ts.as_str() == msg.ts.as_str())
                    {
                        selected_idx = Some(idx);
                    }

                    let who = msg.user.as_ref().map(|u| u.as_str()).unwrap_or("<unknown>");
                    let mut header = format!("{who} {}", msg.ts.as_str());
                    if !msg.reactions.is_empty() {
                        header.push_str("  ");
                        header.push_str(&format_reactions(&msg.reactions));
                    }

                    let mut lines = Vec::<Line>::new();
                    lines.push(Line::from(Span::styled(
                        header,
                        Style::default().add_modifier(Modifier::BOLD),
                    )));

                    let rendered = crate::render::mrkdwn::render_mrkdwn_to_plaintext(&msg.text);
                    for line in rendered.lines() {
                        if line.trim().is_empty() {
                            lines.push(Line::from(""));
                        } else {
                            lines.push(Line::from(Span::raw(format!("  {}", line.trim_end()))));
                        }
                    }

                    for line in crate::render::attachments::render_files_to_plaintext(&msg.files) {
                        lines.push(Line::from(Span::raw(format!("  {}", line.trim_end()))));
                    }

                    // Spacer between messages.
                    lines.push(Line::from(""));

                    items.push(ListItem::new(Text::from(lines)));
                }

                if is_active && selected_idx.is_none() {
                    selected_idx = Some(display.len().saturating_sub(1));
                }

                let highlight_style = Style::default()
                    .add_modifier(Modifier::REVERSED)
                    .fg(Color::Black)
                    .bg(Color::Yellow);

                let list = List::new(items)
                    .highlight_style(highlight_style)
                    .highlight_symbol(">> ");

                let mut list_state =
                    ListState::default().with_selected(if is_active { selected_idx } else { None });

                frame.render_stateful_widget(list, inner, &mut list_state);
            }
        }
    }
}

fn conversation_title(state: &AppState, id: &ConversationId) -> String {
    for c in &state.sidebar.channels {
        if &c.id == id {
            return c.display_title();
        }
    }
    for c in &state.sidebar.dms {
        if &c.id == id {
            return c.display_title();
        }
    }

    id.to_string()
}

fn format_reactions(reactions: &[crate::model::Reaction]) -> String {
    reactions
        .iter()
        .map(|r| {
            let mut s = format!(":{}:", r.name);
            if r.me {
                s.push('*');
            }
            s.push_str(&r.count.to_string());
            s
        })
        .collect::<Vec<_>>()
        .join(" ")
}
