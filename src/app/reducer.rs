use crate::app::action::Action;
use crate::app::effects::Effect;
use crate::app::prompt::PromptState;
use crate::app::state::AppState;
use crate::model::{
    ConversationId, ConversationKind, MessageFile, Reaction, ThreadKey, ThreadSummary,
};
use crate::workspace::ops::{self, FocusDir};
use crate::workspace::tree::PaneKind;
use crate::workspace::tree::SplitAxis;

#[derive(Debug, Default)]
pub struct ReduceOutcome {
    pub should_quit: bool,
    pub effects: Vec<Effect>,
}

pub fn apply_action(state: &mut AppState, action: Action) -> anyhow::Result<ReduceOutcome> {
    let mut out = ReduceOutcome::default();

    match action {
        Action::Quit => {
            out.should_quit = true;
            return Ok(out);
        }
        Action::ToggleFocusArea => match state.focus_area {
            crate::input::keymap::FocusArea::Sidebar => {
                state.focus_area = crate::input::keymap::FocusArea::Workspace;
            }
            crate::input::keymap::FocusArea::Workspace => {
                state.focus_area = crate::input::keymap::FocusArea::Sidebar;
            }
        },
        Action::EnterCompose => {
            state.mode = crate::input::keymap::KeymapMode::Compose;
            state.focus_area = crate::input::keymap::FocusArea::Workspace;
        }
        Action::LeaveCompose => {
            state.mode = crate::input::keymap::KeymapMode::Normal;
        }
        Action::ComposerSend => {
            if state.mode != crate::input::keymap::KeymapMode::Compose {
                anyhow::bail!("cannot send: not in compose mode");
            }

            if !state.shift_enter_confirmed {
                anyhow::bail!(
                    "cannot send yet: we haven't seen a real `Shift+Enter` key event in this session.\n\
                    Press `Shift+Enter` once in the composer to confirm newline support.\n\
                    If you *did* press `Shift+Enter` and still got this error, your terminal is not reporting `Shift+Enter` distinctly.\n\
                    Fix options:\n\
                    - Switch terminals (kitty/wezterm/foot/alacritty), or\n\
                    - Change `keybinds.composer_newline` in `config.toml` (e.g. `Ctrl+j`) so newline doesn't depend on `Shift+Enter`."
                );
            }

            let msg = state.composer.lines().join("\n");
            if msg.trim().is_empty() {
                return Ok(out);
            }

            let (conversation, thread_ts) = match state.workspace.focused_kind() {
                PaneKind::Placeholder => {
                    anyhow::bail!("cannot send: focused pane is empty (open a channel/DM first)");
                }
                PaneKind::Timeline { conversation } => (conversation, None),
                PaneKind::Thread {
                    conversation,
                    thread_ts,
                } => (conversation, Some(thread_ts)),
            };

            state.last_sent = Some(msg.clone());
            state.composer = Default::default();

            out.effects.push(Effect::SlackSendMessage {
                conversation,
                text: msg,
                thread_ts,
            });
        }
        Action::ComposerNewline => {
            if state.mode != crate::input::keymap::KeymapMode::Compose {
                anyhow::bail!("cannot insert newline: not in compose mode");
            }

            state.shift_enter_confirmed = true;
            state.composer.insert_newline();
        }
        Action::SplitVertical => {
            let new_id = ops::split_focused(&mut state.workspace, SplitAxis::Vertical);
            state.pane_views.entry(new_id).or_default();
            state.focus_area = crate::input::keymap::FocusArea::Workspace;
        }
        Action::SplitHorizontal => {
            let new_id = ops::split_focused(&mut state.workspace, SplitAxis::Horizontal);
            state.pane_views.entry(new_id).or_default();
            state.focus_area = crate::input::keymap::FocusArea::Workspace;
        }
        Action::FocusLeft => {
            let id = ops::move_focus(&mut state.workspace, FocusDir::Left);
            state.pane_views.entry(id).or_default();
            state.focus_area = crate::input::keymap::FocusArea::Workspace;
        }
        Action::FocusDown => {
            let id = ops::move_focus(&mut state.workspace, FocusDir::Down);
            state.pane_views.entry(id).or_default();
            state.focus_area = crate::input::keymap::FocusArea::Workspace;
        }
        Action::FocusUp => {
            let id = ops::move_focus(&mut state.workspace, FocusDir::Up);
            state.pane_views.entry(id).or_default();
            state.focus_area = crate::input::keymap::FocusArea::Workspace;
        }
        Action::FocusRight => {
            let id = ops::move_focus(&mut state.workspace, FocusDir::Right);
            state.pane_views.entry(id).or_default();
            state.focus_area = crate::input::keymap::FocusArea::Workspace;
        }
        Action::ClosePane => {
            let old_id = state.workspace.focused();
            if let Some(new_focus) = ops::close_focused(&mut state.workspace) {
                state.pane_views.remove(&old_id);
                state.pane_views.entry(new_focus).or_default();
            }
            state.focus_area = crate::input::keymap::FocusArea::Workspace;
        }
        Action::SidebarRefresh => {
            state.focus_area = crate::input::keymap::FocusArea::Sidebar;
            state.sidebar.loading = true;
            out.effects.push(Effect::SlackLoadSidebar);
        }
        Action::SidebarSelectNext => {
            state.focus_area = crate::input::keymap::FocusArea::Sidebar;
            state.sidebar.select_next();
        }
        Action::SidebarSelectPrev => {
            state.focus_area = crate::input::keymap::FocusArea::Sidebar;
            state.sidebar.select_prev();
        }
        Action::SidebarOpenSelected => {
            if let Some(thread) = state.sidebar.selected_thread_key() {
                return apply_action(
                    state,
                    Action::OpenThread {
                        conversation: thread.conversation,
                        thread_ts: thread.thread_ts,
                    },
                );
            }

            let Some(summary) = state.sidebar.selected_conversation() else {
                return Ok(out);
            };

            if matches!(
                summary.kind,
                ConversationKind::Channel | ConversationKind::PrivateChannel
            ) && !summary.is_member
            {
                anyhow::bail!(
                    "cannot open {}: the configured BOT_TOKEN is not a member of this channel.\n\
                    Fix options:\n\
                    - Invite the bot user to the channel, or\n\
                    - Use a token with access (not recommended unless you know the risks).",
                    summary.display_title()
                );
            }

            let conversation = summary.id.clone();
            state.workspace.set_focused_kind(PaneKind::Timeline {
                conversation: conversation.clone(),
            });
            state.focus_area = crate::input::keymap::FocusArea::Workspace;
            {
                let focused = state.workspace.focused();
                let view = state.pane_views.entry(focused).or_default();
                view.selected_ts = None;
            }

            let timeline = state.timelines.entry(conversation.clone()).or_default();
            timeline.loading = true;
            timeline.messages.clear();
            timeline.next_cursor = None;

            out.effects.push(Effect::SlackLoadHistory {
                conversation,
                cursor: None,
                limit: 50,
                append: false,
            });
        }
        Action::OpenConversation { conversation } => {
            state.workspace.set_focused_kind(PaneKind::Timeline {
                conversation: conversation.clone(),
            });
            state.focus_area = crate::input::keymap::FocusArea::Workspace;
            {
                let focused = state.workspace.focused();
                let view = state.pane_views.entry(focused).or_default();
                view.selected_ts = None;
            }

            let timeline = state.timelines.entry(conversation.clone()).or_default();
            timeline.loading = true;
            timeline.messages.clear();
            timeline.next_cursor = None;

            out.effects.push(Effect::SlackLoadHistory {
                conversation,
                cursor: None,
                limit: 50,
                append: false,
            });
        }
        Action::OpenThread {
            conversation,
            thread_ts,
        } => {
            state.workspace.set_focused_kind(PaneKind::Thread {
                conversation: conversation.clone(),
                thread_ts: thread_ts.clone(),
            });
            state.focus_area = crate::input::keymap::FocusArea::Workspace;

            let focused = state.workspace.focused();
            {
                let view = state.pane_views.entry(focused).or_default();
                view.selected_ts = None;
            }

            let key = ThreadKey {
                conversation: conversation.clone(),
                thread_ts: thread_ts.clone(),
            };

            let title = format!(
                "{} — {}",
                conversation_display_title(state, &conversation),
                thread_ts
            );
            upsert_thread_summary(
                state,
                ThreadSummary {
                    key: key.clone(),
                    title,
                },
            );

            let thread = state.threads.entry(key).or_default();
            thread.loading = true;
            thread.messages.clear();
            thread.next_cursor = None;

            out.effects.push(Effect::SlackLoadThreadReplies {
                conversation,
                thread_ts,
                cursor: None,
                limit: 50,
                append: false,
            });
        }
        Action::OpenThreadFromSelection => {
            let PaneKind::Timeline { conversation } = state.workspace.focused_kind() else {
                return Ok(out);
            };

            let focused = state.workspace.focused();
            let selected_ts = state
                .pane_views
                .get(&focused)
                .and_then(|v| v.selected_ts.as_ref())
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("cannot open thread: no selected message"))?;

            let Some(timeline) = state.timelines.get(&conversation) else {
                anyhow::bail!(
                    "cannot open thread: no timeline loaded for {}",
                    conversation
                );
            };
            let Some(msg) = timeline
                .messages
                .iter()
                .find(|m| m.ts.as_str() == selected_ts.as_str())
            else {
                anyhow::bail!(
                    "cannot open thread: selected message ts {} not found in timeline {}",
                    selected_ts,
                    conversation
                );
            };

            let root_ts = msg.thread_ts.clone().unwrap_or_else(|| msg.ts.clone());
            return apply_action(
                state,
                Action::OpenThread {
                    conversation,
                    thread_ts: root_ts,
                },
            );
        }
        Action::SidebarLoaded { channels, dms } => {
            state.sidebar.channels = channels;
            state.sidebar.dms = dms;
            state.sidebar.loading = false;
            state.sidebar.clamp_selection();
        }
        Action::TimelineLoadOlder => {
            let PaneKind::Timeline { conversation } = state.workspace.focused_kind() else {
                return Ok(out);
            };

            let Some(timeline) = state.timelines.get_mut(&conversation) else {
                return Ok(out);
            };
            if timeline.loading || !timeline.has_more() {
                return Ok(out);
            }

            state.focus_area = crate::input::keymap::FocusArea::Workspace;

            let Some(cursor) = timeline.next_cursor.clone() else {
                return Ok(out);
            };

            timeline.loading = true;

            out.effects.push(Effect::SlackLoadHistory {
                conversation,
                cursor: Some(cursor),
                limit: 50,
                append: true,
            });
        }
        Action::TimelineSelectPrev
        | Action::TimelineSelectNext
        | Action::TimelineSelectFirst
        | Action::TimelineSelectLast => {
            let focused_kind = state.workspace.focused_kind();
            let messages = match &focused_kind {
                PaneKind::Timeline { conversation } => {
                    let Some(timeline) = state.timelines.get(conversation) else {
                        return Ok(out);
                    };
                    &timeline.messages
                }
                PaneKind::Thread {
                    conversation,
                    thread_ts,
                } => {
                    let key = ThreadKey {
                        conversation: conversation.clone(),
                        thread_ts: thread_ts.clone(),
                    };
                    let Some(thread) = state.threads.get(&key) else {
                        return Ok(out);
                    };
                    &thread.messages
                }
                PaneKind::Placeholder => return Ok(out),
            };

            if messages.is_empty() {
                return Ok(out);
            }

            state.focus_area = crate::input::keymap::FocusArea::Workspace;

            let focused = state.workspace.focused();
            let view = state.pane_views.entry(focused).or_default();

            // Display order is oldest → newest.
            let display = messages.iter().rev().collect::<Vec<_>>();
            let selected_idx = view
                .selected_ts
                .as_ref()
                .and_then(|ts| display.iter().position(|m| m.ts.as_str() == ts.as_str()));

            let newest_idx = display.len().saturating_sub(1);
            let current = selected_idx.unwrap_or(newest_idx);

            let next = match action {
                Action::TimelineSelectPrev => current.saturating_sub(1),
                Action::TimelineSelectNext => (current + 1).min(newest_idx),
                Action::TimelineSelectFirst => 0,
                Action::TimelineSelectLast => newest_idx,
                _ => unreachable!("covered by outer match arm"),
            };

            view.selected_ts = Some(display[next].ts.clone());

            match focused_kind {
                PaneKind::Timeline { conversation } => {
                    if let Some(file_id) = selected_file_id_needing_info(state, &conversation, None)
                    {
                        out.effects.push(Effect::SlackLoadFileInfo { file_id });
                    }
                }
                PaneKind::Thread {
                    conversation,
                    thread_ts,
                } => {
                    if let Some(file_id) =
                        selected_file_id_needing_info(state, &conversation, Some(&thread_ts))
                    {
                        out.effects.push(Effect::SlackLoadFileInfo { file_id });
                    }
                }
                PaneKind::Placeholder => {}
            }
        }
        Action::ReactionPromptOpen => {
            state.focus_area = crate::input::keymap::FocusArea::Workspace;

            let focused = state.workspace.focused();
            let selected_ts = state
                .pane_views
                .get(&focused)
                .and_then(|v| v.selected_ts.as_ref())
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("cannot react: no selected message"))?;

            // Fail-fast: ensure we can resolve the selected message in the focused pane.
            match state.workspace.focused_kind() {
                PaneKind::Placeholder => {
                    anyhow::bail!("cannot react: focused pane is empty (open a channel/DM first)");
                }
                PaneKind::Timeline { conversation } => {
                    let Some(timeline) = state.timelines.get(&conversation) else {
                        anyhow::bail!("cannot react: no timeline loaded for {}", conversation);
                    };
                    if !timeline
                        .messages
                        .iter()
                        .any(|m| m.ts.as_str() == selected_ts.as_str())
                    {
                        anyhow::bail!(
                            "cannot react: selected message ts {} not found in timeline {}",
                            selected_ts,
                            conversation
                        );
                    }
                }
                PaneKind::Thread {
                    conversation,
                    thread_ts,
                } => {
                    let key = ThreadKey {
                        conversation: conversation.clone(),
                        thread_ts,
                    };
                    let Some(thread) = state.threads.get(&key) else {
                        anyhow::bail!("cannot react: no thread loaded for {}", key.conversation);
                    };
                    if !thread
                        .messages
                        .iter()
                        .any(|m| m.ts.as_str() == selected_ts.as_str())
                    {
                        anyhow::bail!(
                            "cannot react: selected message ts {} not found in thread {} @ {}",
                            selected_ts,
                            key.conversation,
                            key.thread_ts
                        );
                    }
                }
            }

            state.prompt = Some(PromptState::reaction_emoji(&state.last_reaction_emoji));
        }
        Action::ReactionToggle { emoji } => {
            state.prompt = None;
            state.focus_area = crate::input::keymap::FocusArea::Workspace;

            let emoji = normalize_reaction_emoji(&emoji);
            if emoji.is_empty() {
                anyhow::bail!("cannot react: empty emoji name");
            }
            state.last_reaction_emoji = emoji.clone();

            let focused = state.workspace.focused();
            let selected_ts = state
                .pane_views
                .get(&focused)
                .and_then(|v| v.selected_ts.as_ref())
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("cannot react: no selected message"))?;

            let (conversation, message) = match state.workspace.focused_kind() {
                PaneKind::Placeholder => {
                    anyhow::bail!("cannot react: focused pane is empty (open a channel/DM first)");
                }
                PaneKind::Timeline { conversation } => {
                    let Some(timeline) = state.timelines.get(&conversation) else {
                        anyhow::bail!("cannot react: no timeline loaded for {}", conversation);
                    };
                    let Some(msg) = timeline
                        .messages
                        .iter()
                        .find(|m| m.ts.as_str() == selected_ts.as_str())
                    else {
                        anyhow::bail!(
                            "cannot react: selected message ts {} not found in timeline {}",
                            selected_ts,
                            conversation
                        );
                    };
                    (conversation, msg.clone())
                }
                PaneKind::Thread {
                    conversation,
                    thread_ts,
                } => {
                    let key = ThreadKey {
                        conversation: conversation.clone(),
                        thread_ts,
                    };
                    let Some(thread) = state.threads.get(&key) else {
                        anyhow::bail!(
                            "cannot react: no thread loaded for {} @ {}",
                            key.conversation,
                            key.thread_ts
                        );
                    };
                    let Some(msg) = thread
                        .messages
                        .iter()
                        .find(|m| m.ts.as_str() == selected_ts.as_str())
                    else {
                        anyhow::bail!(
                            "cannot react: selected message ts {} not found in thread {} @ {}",
                            selected_ts,
                            key.conversation,
                            key.thread_ts
                        );
                    };
                    (conversation, msg.clone())
                }
            };

            let already_me = message
                .reactions
                .iter()
                .find(|r| r.name == emoji)
                .is_some_and(|r| r.me);

            if already_me {
                out.effects.push(Effect::SlackRemoveReaction {
                    conversation,
                    ts: selected_ts,
                    emoji,
                });
            } else {
                out.effects.push(Effect::SlackAddReaction {
                    conversation,
                    ts: selected_ts,
                    emoji,
                });
            }
        }
        Action::FileUploadPromptOpen => {
            state.focus_area = crate::input::keymap::FocusArea::Workspace;

            match state.workspace.focused_kind() {
                PaneKind::Placeholder => {
                    anyhow::bail!(
                        "cannot upload: focused pane is empty (open a channel/DM/thread first)"
                    );
                }
                PaneKind::Timeline { .. } | PaneKind::Thread { .. } => {}
            }

            state.prompt = Some(PromptState::file_upload_path(""));
        }
        Action::FileUploadStart { path } => {
            state.prompt = None;
            state.focus_area = crate::input::keymap::FocusArea::Workspace;

            let path = path.trim().to_string();
            if path.is_empty() {
                anyhow::bail!("cannot upload: empty path");
            }

            let (conversation, thread_ts) = match state.workspace.focused_kind() {
                PaneKind::Placeholder => {
                    anyhow::bail!(
                        "cannot upload: focused pane is empty (open a channel/DM/thread first)"
                    );
                }
                PaneKind::Timeline { conversation } => (conversation, None),
                PaneKind::Thread {
                    conversation,
                    thread_ts,
                } => (conversation, Some(thread_ts)),
            };

            out.effects.push(Effect::SlackUploadFile {
                conversation,
                thread_ts,
                path,
            });
        }
        Action::FileUploadCompleted {
            conversation,
            thread_ts,
            files,
        } => {
            state.last_uploaded_file_id = files.first().map(|f| f.id.clone());
            {
                let focused = state.workspace.focused();
                state.pane_views.entry(focused).or_default().selected_ts = None;
            }

            // Refresh views deterministically without requiring Socket Mode to be running.
            out.effects.push(Effect::SlackLoadHistory {
                conversation: conversation.clone(),
                cursor: None,
                limit: 20,
                append: false,
            });

            if let Some(thread_ts) = thread_ts {
                out.effects.push(Effect::SlackLoadThreadReplies {
                    conversation,
                    thread_ts,
                    cursor: None,
                    limit: 50,
                    append: false,
                });
            }
        }
        Action::FileDownloadPromptOpen => {
            state.focus_area = crate::input::keymap::FocusArea::Workspace;

            let focused = state.workspace.focused();
            let selected_ts = state
                .pane_views
                .get(&focused)
                .and_then(|v| v.selected_ts.as_ref())
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("cannot download: no selected message"))?;

            let message = match state.workspace.focused_kind() {
                PaneKind::Placeholder => {
                    anyhow::bail!(
                        "cannot download: focused pane is empty (open a channel/DM first)"
                    );
                }
                PaneKind::Timeline { conversation } => {
                    let Some(timeline) = state.timelines.get(&conversation) else {
                        anyhow::bail!("cannot download: no timeline loaded for {}", conversation);
                    };
                    timeline
                        .messages
                        .iter()
                        .find(|m| m.ts.as_str() == selected_ts.as_str())
                        .cloned()
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "cannot download: selected message ts {} not found in timeline {}",
                                selected_ts,
                                conversation
                            )
                        })?
                }
                PaneKind::Thread {
                    conversation,
                    thread_ts,
                } => {
                    let key = ThreadKey {
                        conversation: conversation.clone(),
                        thread_ts,
                    };
                    let Some(thread) = state.threads.get(&key) else {
                        anyhow::bail!(
                            "cannot download: no thread loaded for {} @ {}",
                            key.conversation,
                            key.thread_ts
                        );
                    };
                    thread
                        .messages
                        .iter()
                        .find(|m| m.ts.as_str() == selected_ts.as_str())
                        .cloned()
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "cannot download: selected message ts {} not found in thread {} @ {}",
                                selected_ts,
                                key.conversation,
                                key.thread_ts
                            )
                        })?
                }
            };

            let file =
                message.files.first().cloned().ok_or_else(|| {
                    anyhow::anyhow!("cannot download: selected message has no files")
                })?;

            let prefill = file.display_name();
            state.prompt = Some(PromptState::file_download_path(&prefill));
        }
        Action::FileDownloadStart { dest_path } => {
            state.prompt = None;
            state.focus_area = crate::input::keymap::FocusArea::Workspace;

            let dest_path = dest_path.trim().to_string();
            if dest_path.is_empty() {
                anyhow::bail!("cannot download: empty destination path");
            }

            let focused = state.workspace.focused();
            let selected_ts = state
                .pane_views
                .get(&focused)
                .and_then(|v| v.selected_ts.as_ref())
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("cannot download: no selected message"))?;

            let files = match state.workspace.focused_kind() {
                PaneKind::Placeholder => {
                    anyhow::bail!(
                        "cannot download: focused pane is empty (open a channel/DM first)"
                    );
                }
                PaneKind::Timeline { conversation } => {
                    let Some(timeline) = state.timelines.get(&conversation) else {
                        anyhow::bail!("cannot download: no timeline loaded for {}", conversation);
                    };
                    timeline
                        .messages
                        .iter()
                        .find(|m| m.ts.as_str() == selected_ts.as_str())
                        .map(|m| m.files.clone())
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "cannot download: selected message ts {} not found in timeline {}",
                                selected_ts,
                                conversation
                            )
                        })?
                }
                PaneKind::Thread {
                    conversation,
                    thread_ts,
                } => {
                    let key = ThreadKey {
                        conversation: conversation.clone(),
                        thread_ts,
                    };
                    let Some(thread) = state.threads.get(&key) else {
                        anyhow::bail!(
                            "cannot download: no thread loaded for {} @ {}",
                            key.conversation,
                            key.thread_ts
                        );
                    };
                    thread
                        .messages
                        .iter()
                        .find(|m| m.ts.as_str() == selected_ts.as_str())
                        .map(|m| m.files.clone())
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "cannot download: selected message ts {} not found in thread {} @ {}",
                                selected_ts,
                                key.conversation,
                                key.thread_ts
                            )
                        })?
                }
            };

            let url = files
                .iter()
                .find_map(|f| f.url_private_download.as_ref().cloned())
                .ok_or_else(|| {
                    anyhow::anyhow!("cannot download: file has no url_private_download")
                })?;

            out.effects
                .push(Effect::SlackDownloadFile { url, dest_path });
        }
        Action::FileDownloaded { dest_path, bytes } => {
            state.last_downloaded_path = Some(dest_path);
            state.last_downloaded_bytes = Some(bytes);
        }
        Action::FileInfoLoaded {
            file_id,
            size,
            mode,
            preview_plain_text,
        } => {
            apply_file_info_to_messages(
                state.timelines.values_mut().map(|t| &mut t.messages),
                &file_id,
                &size,
                mode.as_deref(),
                preview_plain_text.as_deref(),
            );
            apply_file_info_to_messages(
                state.threads.values_mut().map(|t| &mut t.messages),
                &file_id,
                &size,
                mode.as_deref(),
                preview_plain_text.as_deref(),
            );
        }
        Action::TimelineLoaded {
            conversation,
            messages,
            next_cursor,
            append,
        } => {
            let conv_id = conversation.clone();
            let timeline = state.timelines.entry(conversation).or_default();
            timeline.loading = false;

            if append {
                let mut existing = std::collections::HashSet::new();
                for msg in &timeline.messages {
                    existing.insert(msg.ts.clone());
                }

                for msg in messages {
                    if existing.insert(msg.ts.clone()) {
                        timeline.messages.push(msg);
                    }
                }
            } else if timeline.messages.is_empty() {
                timeline.messages = messages;
            } else {
                // Fail-safe merge: avoid dropping Socket Mode-delivered messages that arrived while a
                // history fetch was in flight.
                let mut existing = std::collections::HashSet::new();
                for msg in &timeline.messages {
                    existing.insert(msg.ts.clone());
                }

                for msg in messages {
                    if existing.insert(msg.ts.clone()) {
                        timeline.messages.push(msg);
                    }
                }
            }

            timeline.next_cursor = next_cursor;

            if let Some(newest) = timeline.messages.first().map(|m| m.ts.clone()) {
                for (pane_id, kind) in state.workspace.leaf_kinds() {
                    if let PaneKind::Timeline { conversation } = kind {
                        if conversation == conv_id {
                            let view = state.pane_views.entry(pane_id).or_default();
                            if view.selected_ts.is_none() {
                                view.selected_ts = Some(newest.clone());
                            }
                        }
                    }
                }
            }

            // If the focused (or newly selected) message includes a file, lazily pull file info so
            // size/snippet preview can be rendered without blocking the UI render path.
            if let PaneKind::Timeline { conversation } = state.workspace.focused_kind() {
                if conversation == conv_id {
                    if let Some(file_id) = selected_file_id_needing_info(state, &conv_id, None) {
                        out.effects.push(Effect::SlackLoadFileInfo { file_id });
                    }
                }
            }
        }
        Action::ThreadLoaded {
            conversation,
            thread_ts,
            messages,
            next_cursor,
            append,
        } => {
            let key = ThreadKey {
                conversation: conversation.clone(),
                thread_ts: thread_ts.clone(),
            };
            let thread = state.threads.entry(key.clone()).or_default();
            thread.loading = false;

            if append {
                let mut existing = std::collections::HashSet::new();
                for msg in &thread.messages {
                    existing.insert(msg.ts.clone());
                }

                for msg in messages {
                    if existing.insert(msg.ts.clone()) {
                        thread.messages.push(msg);
                    }
                }
            } else {
                thread.messages = messages;
            }

            thread.next_cursor = next_cursor;

            if let Some(newest) = thread.messages.first().map(|m| m.ts.clone()) {
                for (pane_id, kind) in state.workspace.leaf_kinds() {
                    if let PaneKind::Thread {
                        conversation,
                        thread_ts,
                    } = kind
                    {
                        if conversation == key.conversation && thread_ts == key.thread_ts {
                            let view = state.pane_views.entry(pane_id).or_default();
                            if view.selected_ts.is_none() {
                                view.selected_ts = Some(newest.clone());
                            }
                        }
                    }
                }
            }

            if let PaneKind::Thread {
                conversation,
                thread_ts,
            } = state.workspace.focused_kind()
            {
                if conversation == key.conversation && thread_ts == key.thread_ts {
                    if let Some(file_id) = selected_file_id_needing_info(
                        state,
                        &key.conversation,
                        Some(&key.thread_ts),
                    ) {
                        out.effects.push(Effect::SlackLoadFileInfo { file_id });
                    }
                }
            }
        }
        Action::ReactionChanged {
            conversation,
            ts,
            emoji,
            delta,
            me,
        } => {
            if let Some(timeline) = state.timelines.get_mut(&conversation) {
                for msg in &mut timeline.messages {
                    if msg.ts.as_str() == ts.as_str() {
                        apply_reaction_delta(msg, &emoji, delta, me);
                    }
                }
            }

            for (key, thread) in &mut state.threads {
                if key.conversation != conversation {
                    continue;
                }
                for msg in &mut thread.messages {
                    if msg.ts.as_str() == ts.as_str() {
                        apply_reaction_delta(msg, &emoji, delta, me);
                    }
                }
            }
        }
        Action::SlackMessageReceived {
            conversation,
            message,
        } => {
            if let Some(timeline) = state.timelines.get_mut(&conversation) {
                if !timeline
                    .messages
                    .iter()
                    .any(|m| m.ts.as_str() == message.ts.as_str())
                {
                    // Slack history is newest-first; keep the same invariant for incremental updates.
                    timeline.messages.insert(0, message.clone());
                }
            }

            if let Some(root_ts) = message.thread_ts.as_ref() {
                let key = ThreadKey {
                    conversation: conversation.clone(),
                    thread_ts: root_ts.clone(),
                };
                if let Some(thread) = state.threads.get_mut(&key) {
                    if !thread
                        .messages
                        .iter()
                        .any(|m| m.ts.as_str() == message.ts.as_str())
                    {
                        thread.messages.insert(0, message);
                    }

                    let title = format!(
                        "{} — {}",
                        conversation_display_title(state, &key.conversation),
                        key.thread_ts
                    );
                    upsert_thread_summary(state, ThreadSummary { key, title });
                }
            }
        }
    }

    Ok(out)
}

fn conversation_display_title(state: &AppState, id: &ConversationId) -> String {
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

fn upsert_thread_summary(state: &mut AppState, summary: ThreadSummary) {
    // Preserve selection identity across thread list mutations so opening a thread doesn't
    // unexpectedly change the selected channel/DM in the sidebar.
    let selected_thread = state.sidebar.selected_thread_key();
    let selected_conversation = state.sidebar.selected_conversation_id();

    state.sidebar.threads.retain(|t| t.key != summary.key);
    state.sidebar.threads.insert(0, summary);

    if let Some(key) = selected_thread {
        state.sidebar.selected_idx = state.sidebar.threads.iter().position(|t| t.key == key);
        return;
    }

    if let Some(conv) = selected_conversation {
        let threads_len = state.sidebar.threads.len();
        if let Some(idx) = state.sidebar.channels.iter().position(|c| c.id == conv) {
            state.sidebar.selected_idx = Some(threads_len + idx);
            return;
        }
        if let Some(idx) = state.sidebar.dms.iter().position(|c| c.id == conv) {
            state.sidebar.selected_idx = Some(threads_len + state.sidebar.channels.len() + idx);
            return;
        }
    }

    state.sidebar.clamp_selection();
}

fn normalize_reaction_emoji(raw: &str) -> String {
    let mut s = raw.trim().to_string();
    if let Some(stripped) = s.strip_prefix(':').and_then(|t| t.strip_suffix(':')) {
        s = stripped.to_string();
    }
    s.trim().to_string()
}

fn apply_reaction_delta(
    msg: &mut crate::model::Message,
    emoji: &str,
    delta: i64,
    me: Option<bool>,
) {
    if delta == 0 {
        return;
    }

    let idx = msg.reactions.iter().position(|r| r.name == emoji);
    match idx {
        Some(i) => {
            let reaction = &mut msg.reactions[i];
            let count_i64 = reaction.count as i64;
            let next = (count_i64 + delta).max(0) as u64;
            reaction.count = next;
            if let Some(me) = me {
                reaction.me = me;
            }
            if reaction.count == 0 {
                msg.reactions.remove(i);
            }
        }
        None => {
            if delta > 0 {
                msg.reactions.push(Reaction {
                    name: emoji.to_string(),
                    count: delta as u64,
                    me: me.unwrap_or(false),
                });
            }
        }
    }
}

fn apply_file_info_to_messages<'a, I>(
    message_lists: I,
    file_id: &str,
    size: &Option<u64>,
    mode: Option<&str>,
    preview_plain_text: Option<&str>,
) where
    I: IntoIterator<Item = &'a mut Vec<crate::model::Message>>,
{
    for messages in message_lists {
        for msg in messages.iter_mut() {
            for f in msg.files.iter_mut() {
                if f.id != file_id {
                    continue;
                }

                if f.size.is_none() {
                    f.size = *size;
                }
                if f.mode.is_none() {
                    f.mode = mode.map(|s| s.to_string());
                }

                if f.mode.as_deref() == Some("snippet") && f.snippet_preview.is_none() {
                    f.snippet_preview = Some(preview_plain_text.unwrap_or("").to_string());
                }
            }
        }
    }
}

fn selected_file_id_needing_info(
    state: &AppState,
    conversation: &ConversationId,
    thread_ts: Option<&crate::model::MessageTs>,
) -> Option<String> {
    let focused = state.workspace.focused();
    let selected_ts = state
        .pane_views
        .get(&focused)
        .and_then(|v| v.selected_ts.as_ref())?;

    let msg = match thread_ts {
        None => state.timelines.get(conversation).and_then(|t| {
            t.messages
                .iter()
                .find(|m| m.ts.as_str() == selected_ts.as_str())
        })?,
        Some(root_ts) => {
            let key = ThreadKey {
                conversation: conversation.clone(),
                thread_ts: root_ts.clone(),
            };
            state.threads.get(&key).and_then(|t| {
                t.messages
                    .iter()
                    .find(|m| m.ts.as_str() == selected_ts.as_str())
            })?
        }
    };

    msg.files
        .iter()
        .find(|f| file_needs_info(f))
        .map(|f| f.id.clone())
}

fn file_needs_info(f: &MessageFile) -> bool {
    if f.mode.is_none() {
        return true;
    }

    f.mode.as_deref() == Some("snippet") && f.snippet_preview.is_none()
}
