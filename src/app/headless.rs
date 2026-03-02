use std::io::{self, Read as _};
use std::time::Duration;

use anyhow::Context as _;
use serde_json::json;

use crate::app::action::Action;
use crate::app::effects::{self, Effect};
use crate::app::reducer;
use crate::app::state::AppState;
use crate::input::keymap::KeymapMode;
use crate::model::{ConversationId, MessageTs};
use crate::slack::service::AuthTestSummary;
use crate::slack::service::SlackService;
use crate::slack::socket_mode;
use crate::slack::tokens::SlackTokens;

const DEFAULT_SOCKET_MODE_SMOKE_TIMEOUT_MS: u64 = 5_000;

pub struct HeadlessArgs {
    /// Path to a newline-delimited command script. Use `-` for stdin.
    pub script_path: String,
}

pub fn run_headless(args: HeadlessArgs) -> anyhow::Result<()> {
    crate::diagnostics::log::init_tracing();
    crate::terminal::mark_headless_mode();

    println!(
        "{}",
        json!({
            "event": "started",
            "mode": "headless",
            "script": args.script_path,
        })
    );

    let script = read_script(&args.script_path).context("read headless script")?;

    // Headless runs without a terminal, so the Shift+Enter safety gate is irrelevant here.
    let mut state = AppState {
        shift_enter_confirmed: true,
        ..Default::default()
    };

    let mut slack: Option<SlackHarness> = None;

    for (idx, raw_line) in script.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let cmd = parse_command_line(line).with_context(|| format!("script line {line_no}"))?;
        let should_quit = match cmd {
            HeadlessCommand::Action(action) => {
                let outcome = reducer::apply_action(&mut state, action)
                    .with_context(|| format!("line {line_no}"))?;

                if outcome.should_quit {
                    true
                } else {
                    run_effects_to_completion(&mut state, outcome.effects, &mut slack)
                        .with_context(|| format!("line {line_no} (effects)"))?
                }
            }
            HeadlessCommand::TypeText { text } => {
                if state.mode != KeymapMode::Compose {
                    anyhow::bail!(
                        "type requires COMPOSE mode; run `compose` (enter_compose) first"
                    );
                }

                let text = unescape_script_text(&text);
                state.composer.insert_str(text);
                false
            }
            HeadlessCommand::SlackAuthTest => {
                let slack = get_or_init_slack(&mut slack).context("init Slack harness")?;
                let summary = slack
                    .auth_test()
                    .with_context(|| format!("line {line_no} (Slack auth.test)"))?;
                print_auth_test_summary(&summary);
                false
            }
            HeadlessCommand::SlackSocketModeSmoke { timeout_ms } => {
                let slack = get_or_init_slack(&mut slack).context("init Slack harness")?;
                slack
                    .socket_mode_smoke(Duration::from_millis(timeout_ms))
                    .with_context(|| format!("line {line_no} (Socket Mode smoke)"))?;

                println!(
                    "{}",
                    json!({
                        "event": "slack_socket_mode_smoke",
                        "timeout_ms": timeout_ms,
                        "saw_hello": true,
                        "result": "ok",
                    })
                );

                false
            }
            HeadlessCommand::SlackSocketModeListen { duration_ms } => {
                let harness = get_or_init_slack(&mut slack).context("init Slack harness")?;
                let listen = harness
                    .socket_mode_listen(Duration::from_millis(duration_ms))
                    .with_context(|| format!("line {line_no} (Socket Mode listen)"))?;

                let mut actions_applied = 0_u64;
                let mut should_quit = false;
                for action in listen.actions {
                    let outcome = reducer::apply_action(&mut state, action)
                        .with_context(|| format!("line {line_no} (apply listen action)"))?;
                    actions_applied = actions_applied.saturating_add(1);

                    if outcome.should_quit {
                        should_quit = true;
                        break;
                    }

                    if run_effects_to_completion(&mut state, outcome.effects, &mut slack)
                        .with_context(|| format!("line {line_no} (listen effects)"))?
                    {
                        should_quit = true;
                        break;
                    }
                }

                println!(
                    "{}",
                    json!({
                        "event": "slack_socket_mode_listen",
                        "duration_ms": duration_ms,
                        "saw_hello": listen.saw_hello,
                        "envelopes_seen": listen.envelopes_seen,
                        "actions_applied": actions_applied,
                        "result": "ok",
                    })
                );

                should_quit
            }
        };

        let focused_kind = state.workspace.focused_kind();
        let (focused_conversation, focused_thread_ts) = match &focused_kind {
            crate::workspace::tree::PaneKind::Placeholder => (None, None),
            crate::workspace::tree::PaneKind::Timeline { conversation } => {
                (Some(conversation.clone()), None)
            }
            crate::workspace::tree::PaneKind::Thread {
                conversation,
                thread_ts,
            } => (Some(conversation.clone()), Some(thread_ts.clone())),
        };

        let focused_timeline = matches!(
            &focused_kind,
            crate::workspace::tree::PaneKind::Timeline { .. }
        )
        .then(|| {
            focused_conversation
                .as_ref()
                .and_then(|id| state.timelines.get(id))
        })
        .flatten();
        let focused_timeline_messages = focused_timeline.map(|t| t.messages.len());
        let focused_timeline_loading = focused_timeline.map(|t| t.loading);
        let focused_timeline_has_more = focused_timeline.map(|t| t.has_more());
        let focused_timeline_next_cursor_present =
            focused_timeline.map(|t| t.next_cursor.is_some());

        let focused_thread = focused_thread_ts.as_ref().and_then(|root| {
            focused_conversation.as_ref().and_then(|conv| {
                state.threads.get(&crate::model::ThreadKey {
                    conversation: conv.clone(),
                    thread_ts: root.clone(),
                })
            })
        });
        let focused_thread_messages = focused_thread.map(|t| t.messages.len());
        let focused_thread_loading = focused_thread.map(|t| t.loading);
        let focused_thread_has_more = focused_thread.map(|t| t.has_more());
        let focused_thread_next_cursor_present = focused_thread.map(|t| t.next_cursor.is_some());
        let focused_timeline_selected_ts = state
            .pane_views
            .get(&state.workspace.focused())
            .and_then(|v| v.selected_ts.as_ref())
            .map(|ts| ts.to_string());

        let focused_selected_reactions = state
            .pane_views
            .get(&state.workspace.focused())
            .and_then(|v| v.selected_ts.as_ref())
            .and_then(|selected_ts| match &focused_kind {
                crate::workspace::tree::PaneKind::Timeline { conversation } => state
                    .timelines
                    .get(conversation)
                    .and_then(|t| {
                        t.messages
                            .iter()
                            .find(|m| m.ts.as_str() == selected_ts.as_str())
                    })
                    .map(|m| &m.reactions),
                crate::workspace::tree::PaneKind::Thread {
                    conversation,
                    thread_ts,
                } => state
                    .threads
                    .get(&crate::model::ThreadKey {
                        conversation: conversation.clone(),
                        thread_ts: thread_ts.clone(),
                    })
                    .and_then(|t| {
                        t.messages
                            .iter()
                            .find(|m| m.ts.as_str() == selected_ts.as_str())
                    })
                    .map(|m| &m.reactions),
                crate::workspace::tree::PaneKind::Placeholder => None,
            })
            .map(|reactions| {
                reactions
                    .iter()
                    .map(|r| {
                        json!({
                            "name": r.name.clone(),
                            "count": r.count,
                            "me": r.me,
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let focused_selected_files = state
            .pane_views
            .get(&state.workspace.focused())
            .and_then(|v| v.selected_ts.as_ref())
            .and_then(|selected_ts| match &focused_kind {
                crate::workspace::tree::PaneKind::Timeline { conversation } => state
                    .timelines
                    .get(conversation)
                    .and_then(|t| {
                        t.messages
                            .iter()
                            .find(|m| m.ts.as_str() == selected_ts.as_str())
                    })
                    .map(|m| &m.files),
                crate::workspace::tree::PaneKind::Thread {
                    conversation,
                    thread_ts,
                } => state
                    .threads
                    .get(&crate::model::ThreadKey {
                        conversation: conversation.clone(),
                        thread_ts: thread_ts.clone(),
                    })
                    .and_then(|t| {
                        t.messages
                            .iter()
                            .find(|m| m.ts.as_str() == selected_ts.as_str())
                    })
                    .map(|m| &m.files),
                crate::workspace::tree::PaneKind::Placeholder => None,
            })
            .map(|files| {
                files
                    .iter()
                    .map(|f| {
                        json!({
                            "id": f.id.clone(),
                            "name": f.name.clone(),
                            "title": f.title.clone(),
                            "size": f.size,
                            "mode": f.mode.clone(),
                            "has_preview": f.snippet_preview.as_ref().is_some_and(|s| !s.trim().is_empty()),
                            "has_url_private_download": f.url_private_download.is_some(),
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let selected = state.sidebar.selected_conversation_id();
        let selected_timeline = selected.as_ref().and_then(|id| state.timelines.get(id));
        let selected_timeline_messages = selected_timeline.map(|t| t.messages.len());
        let selected_timeline_loading = selected_timeline.map(|t| t.loading);
        let selected_timeline_has_more = selected_timeline.map(|t| t.has_more());
        let selected_timeline_next_cursor_present =
            selected_timeline.map(|t| t.next_cursor.is_some());

        println!(
            "{}",
            json!({
                "event": "state",
                "line": line_no,
                "mode": match state.mode {
                    KeymapMode::Normal => "NORMAL",
                    KeymapMode::Compose => "COMPOSE",
                },
                "focus_area": match state.focus_area {
                    crate::input::keymap::FocusArea::Sidebar => "SIDEBAR",
                    crate::input::keymap::FocusArea::Workspace => "WORKSPACE",
                },
                "pane_count": state.workspace.leaf_ids().len(),
                "focused_pane": state.workspace.focused().get(),
                "focused_pane_kind": match &focused_kind {
                    crate::workspace::tree::PaneKind::Placeholder => "PLACEHOLDER",
                    crate::workspace::tree::PaneKind::Timeline { .. } => "TIMELINE",
                    crate::workspace::tree::PaneKind::Thread { .. } => "THREAD",
                },
                "focused_conversation": focused_conversation.as_ref().map(|id| id.to_string()),
                "focused_thread_ts": focused_thread_ts.as_ref().map(|ts| ts.to_string()),
                "focused_timeline_messages": focused_timeline_messages,
                "focused_timeline_loading": focused_timeline_loading,
                "focused_timeline_has_more": focused_timeline_has_more,
                "focused_timeline_next_cursor_present": focused_timeline_next_cursor_present,
                "focused_timeline_selected_ts": focused_timeline_selected_ts,
                "focused_selected_reactions": focused_selected_reactions,
                "focused_thread_messages": focused_thread_messages,
                "focused_thread_loading": focused_thread_loading,
                "focused_thread_has_more": focused_thread_has_more,
                "focused_thread_next_cursor_present": focused_thread_next_cursor_present,
                "sidebar_channels": state.sidebar.channels.len(),
                "sidebar_dms": state.sidebar.dms.len(),
                "sidebar_threads": state.sidebar.threads.len(),
                "sidebar_selected_idx": state.sidebar.selected_idx,
                "sidebar_selected_conversation": selected.map(|id| id.to_string()),
                "sidebar_selected_thread": state.sidebar.selected_thread_key().map(|t| json!({"conversation": t.conversation.to_string(), "thread_ts": t.thread_ts.to_string()})),
                "sidebar_selected_timeline_messages": selected_timeline_messages,
                "sidebar_selected_timeline_loading": selected_timeline_loading,
                "sidebar_selected_timeline_has_more": selected_timeline_has_more,
                "sidebar_selected_timeline_next_cursor_present": selected_timeline_next_cursor_present,
                "timelines": state.timelines.len(),
                "threads": state.threads.len(),
                "focused_selected_files": focused_selected_files,
                "last_uploaded_file_id": state.last_uploaded_file_id,
                "last_downloaded_path": state.last_downloaded_path,
                "last_downloaded_bytes": state.last_downloaded_bytes,
            })
        );

        if should_quit {
            println!("{}", json!({ "event": "quit", "line": line_no }));
            return Ok(());
        }
    }

    anyhow::bail!(
        "headless script ended without an explicit quit; add a final `quit` command to make the run deterministic"
    );
}

enum HeadlessCommand {
    Action(Action),
    TypeText { text: String },
    SlackAuthTest,
    SlackSocketModeSmoke { timeout_ms: u64 },
    SlackSocketModeListen { duration_ms: u64 },
}

fn read_script(path: &str) -> anyhow::Result<String> {
    if path == "-" {
        let mut buf = String::new();
        io::stdin()
            .read_to_string(&mut buf)
            .context("read script from stdin")?;
        return Ok(buf);
    }

    std::fs::read_to_string(path).with_context(|| format!("read script file: {path}"))
}

fn parse_command_line(line: &str) -> anyhow::Result<HeadlessCommand> {
    let (keyword, rest) = line
        .split_once(' ')
        .map(|(a, b)| (a.trim(), b.trim()))
        .unwrap_or((line.trim(), ""));

    let norm_keyword = normalize_word(keyword);
    match norm_keyword.as_str() {
        "slackauthtest" => return Ok(HeadlessCommand::SlackAuthTest),
        "slacksocketmodesmoke" => {
            let timeout_ms = if rest.is_empty() {
                DEFAULT_SOCKET_MODE_SMOKE_TIMEOUT_MS
            } else {
                rest.parse::<u64>()
                    .with_context(|| format!("invalid slack_socket_mode_smoke timeout: {rest}"))?
            };
            return Ok(HeadlessCommand::SlackSocketModeSmoke { timeout_ms });
        }
        "slacksocketmodelisten" => {
            let duration_ms = if rest.is_empty() {
                DEFAULT_SOCKET_MODE_SMOKE_TIMEOUT_MS
            } else {
                rest.parse::<u64>()
                    .with_context(|| format!("invalid slack_socket_mode_listen duration: {rest}"))?
            };
            return Ok(HeadlessCommand::SlackSocketModeListen { duration_ms });
        }
        "type" | "insert" | "input" | "composertype" => {
            if rest.is_empty() {
                anyhow::bail!("type requires text, e.g. `type hello world`");
            }
            return Ok(HeadlessCommand::TypeText {
                text: rest.to_string(),
            });
        }
        "react" | "reaction" => {
            if rest.is_empty() {
                anyhow::bail!(
                    "react requires an emoji name, e.g. `react eyes` or `react :thumbsup:`"
                );
            }
            return Ok(HeadlessCommand::Action(Action::ReactionToggle {
                emoji: rest.to_string(),
            }));
        }
        "upload" | "fileupload" => {
            if rest.is_empty() {
                anyhow::bail!("upload requires a path, e.g. `upload ./hello.txt`");
            }
            return Ok(HeadlessCommand::Action(Action::FileUploadStart {
                path: rest.to_string(),
            }));
        }
        "download" | "filedownload" => {
            if rest.is_empty() {
                anyhow::bail!("download requires a destination path, e.g. `download ./out.bin`");
            }
            return Ok(HeadlessCommand::Action(Action::FileDownloadStart {
                dest_path: rest.to_string(),
            }));
        }
        "openconversation" | "openconv" => {
            if rest.is_empty() {
                anyhow::bail!("open_conversation requires a conversation id, e.g. `open_conversation C123...`");
            }
            return Ok(HeadlessCommand::Action(Action::OpenConversation {
                conversation: ConversationId::from(rest),
            }));
        }
        "openthread" | "threadopen" => {
            let mut parts = rest.split_whitespace();
            let conversation = parts
                .next()
                .ok_or_else(|| anyhow::anyhow!("open_thread requires a conversation id and thread ts, e.g. `open_thread C123... 123.456`"))?;
            let thread_ts = parts.next().ok_or_else(|| {
                anyhow::anyhow!(
                    "open_thread requires a conversation id and thread ts, e.g. `open_thread C123... 123.456`"
                )
            })?;
            if parts.next().is_some() {
                anyhow::bail!("open_thread expects exactly 2 arguments: conversation_id thread_ts");
            }

            return Ok(HeadlessCommand::Action(Action::OpenThread {
                conversation: ConversationId::from(conversation),
                thread_ts: MessageTs::from(thread_ts),
            }));
        }
        "open" if !rest.is_empty() => {
            return Ok(HeadlessCommand::Action(Action::OpenConversation {
                conversation: ConversationId::from(rest),
            }));
        }
        _ => {}
    }

    let raw_action = if norm_keyword == "action" { rest } else { line };

    Ok(HeadlessCommand::Action(parse_action_name(raw_action)?))
}

fn parse_action_name(raw: &str) -> anyhow::Result<Action> {
    let norm = normalize_word(raw);
    let action = match norm.as_str() {
        "quit" | "q" => Action::Quit,
        "togglefocusarea" | "togglefocus" | "tab" => Action::ToggleFocusArea,
        "entercompose" | "compose" | "composeenter" => Action::EnterCompose,
        "leavecompose" | "esc" | "normal" => Action::LeaveCompose,
        "composersend" | "send" => Action::ComposerSend,
        "composernewline" | "newline" => Action::ComposerNewline,
        "openthreadfromselection" | "thread" | "threadopenselected" => {
            Action::OpenThreadFromSelection
        }
        "splitvertical" | "splitv" | "split_v" => Action::SplitVertical,
        "splithorizontal" | "splith" | "split_h" => Action::SplitHorizontal,
        "focusleft" | "left" | "h" => Action::FocusLeft,
        "focusdown" | "down" | "j" => Action::FocusDown,
        "focusup" | "up" | "k" => Action::FocusUp,
        "focusright" | "right" | "l" => Action::FocusRight,
        "closepane" | "close" | "x" => Action::ClosePane,
        "resizeverticalplus" | "resizevplus" => Action::ResizeVerticalPlus,
        "resizeverticalminus" | "resizevminus" => Action::ResizeVerticalMinus,
        "resizehorizontalplus" | "resizehplus" => Action::ResizeHorizontalPlus,
        "resizehorizontalminus" | "resizehminus" => Action::ResizeHorizontalMinus,
        "sidebarrefresh" | "refreshsidebar" | "refresh" => Action::SidebarRefresh,
        "sidebarselectnext" | "sidebarnext" | "sidebardown" => Action::SidebarSelectNext,
        "sidebarselectprev" | "sidebarprev" | "sidebarup" => Action::SidebarSelectPrev,
        "sidebaropenselected" | "sidebaropen" | "open" => Action::SidebarOpenSelected,
        "timelineloadolder" | "loadolder" | "older" | "pageup" => Action::TimelineLoadOlder,
        "timelineselectprev" | "timelineup" => Action::TimelineSelectPrev,
        "timelineselectnext" | "timelinedown" => Action::TimelineSelectNext,
        "timelineselectfirst" | "timelinetop" | "gg" => Action::TimelineSelectFirst,
        "timelineselectlast" | "timelinebottom" | "g" => Action::TimelineSelectLast,
        _ => anyhow::bail!("unknown action: {raw}"),
    };

    Ok(action)
}

fn normalize_word(raw: &str) -> String {
    raw.chars()
        .filter(|c| !c.is_whitespace() && *c != '-' && *c != '_')
        .flat_map(|c| c.to_lowercase())
        .collect()
}

fn unescape_script_text(raw: &str) -> String {
    let mut out = String::new();
    let mut it = raw.chars();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }

        match it.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('\\') => out.push('\\'),
            Some('"') => out.push('"'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }

    out
}

fn get_or_init_slack(slack: &mut Option<SlackHarness>) -> anyhow::Result<&SlackHarness> {
    if slack.is_none() {
        *slack = Some(SlackHarness::from_env()?);
    }

    Ok(slack.as_ref().expect("slack harness just initialized"))
}

fn print_auth_test_summary(summary: &AuthTestSummary) {
    println!(
        "{}",
        json!({
            "event": "slack_auth_test",
            "team": summary.team,
            "team_id": summary.team_id,
            "user_id": summary.user_id,
            "user": summary.user,
        })
    );
}

struct SlackHarness {
    rt: Option<tokio::runtime::Runtime>,
    service: SlackService,
}

impl SlackHarness {
    fn from_env() -> anyhow::Result<Self> {
        let tokens = SlackTokens::from_env().context("load Slack tokens from env")?;
        let service = SlackService::new(tokens).context("init Slack service")?;
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .context("init tokio runtime for Slack calls")?;

        Ok(Self {
            rt: Some(rt),
            service,
        })
    }

    fn auth_test(&self) -> anyhow::Result<AuthTestSummary> {
        let timeout = effects::DEFAULT_SLACK_EFFECT_TIMEOUT;
        self.rt
            .as_ref()
            .expect("runtime initialized")
            .block_on(async {
                match tokio::time::timeout(timeout, self.service.auth_test_bot()).await {
                    Ok(v) => v,
                    Err(_) => anyhow::bail!("Slack auth.test timed out after {timeout:?}"),
                }
            })
    }

    fn socket_mode_smoke(&self, timeout: Duration) -> anyhow::Result<()> {
        self.rt
            .as_ref()
            .expect("runtime initialized")
            .block_on(socket_mode::smoke_connect(self.service.clone(), timeout))
    }

    fn socket_mode_listen(&self, duration: Duration) -> anyhow::Result<socket_mode::ListenResult> {
        self.rt
            .as_ref()
            .expect("runtime initialized")
            .block_on(socket_mode::listen_for_actions(
                self.service.clone(),
                duration,
            ))
    }

    fn run_effect(&self, effect: Effect) -> anyhow::Result<Vec<Action>> {
        let effect_debug = format!("{effect:?}");
        let timeout = effects::timeout_for_effect(&effect);
        self.rt
            .as_ref()
            .expect("runtime initialized")
            .block_on(async {
                match tokio::time::timeout(timeout, effects::execute(effect, self.service.clone()))
                    .await
                {
                    Ok(v) => v,
                    Err(_) => {
                        anyhow::bail!("Slack effect timed out after {timeout:?}: {effect_debug}")
                    }
                }
            })
    }
}

impl Drop for SlackHarness {
    fn drop(&mut self) {
        if let Some(rt) = self.rt.take() {
            rt.shutdown_background();
        }
    }
}

fn run_effects_to_completion(
    state: &mut AppState,
    effects: Vec<Effect>,
    slack: &mut Option<SlackHarness>,
) -> anyhow::Result<bool> {
    let mut pending = effects;
    while let Some(effect) = pending.pop() {
        let slack = get_or_init_slack(slack).context("init Slack harness for effects")?;
        let actions = slack.run_effect(effect).context("execute effect")?;
        for action in actions {
            let outcome = reducer::apply_action(state, action).context("apply effect action")?;
            if outcome.should_quit {
                return Ok(true);
            }
            pending.extend(outcome.effects);
        }
    }

    Ok(false)
}
