---
title: "slack-rs — Terminal-Driven Slack TUI — Worklog"
date: 2026-03-01
status: active
related:
  - docs/TERMINAL_DRIVEN_RUST_SLACK_TUI_WITH_VIM_KEYBINDINGS_2026-03-01.md
---

# Worklog

Plan doc (SSOT): `docs/TERMINAL_DRIVEN_RUST_SLACK_TUI_WITH_VIM_KEYBINDINGS_2026-03-01.md`

## Phase 0 (Preflight) Progress Update
- Work completed:
  - Initialized worklog + linked from plan doc.
- Tests run + results:
  - N/A (no code yet)
- Issues / deviations:
  - Repo has no commits yet (unborn `main`); created a local feature branch `feat/tui-skeleton` anyway to avoid implementing on `main`.
- Next steps:
  - Implement Phase 0 crate skeleton (`Cargo.toml` + baseline deps) and compile-only module tree.

## Phase 0 (Preflight) Progress Update
- Work completed:
  - Added `Cargo.toml` + baseline dependencies and a library+binary skeleton.
  - Created the compile-only module tree (`src/app`, `src/ui`, `src/input`, `src/workspace`, `src/slack`, `src/render`, `src/diagnostics`, `src/terminal`).
- Tests run + results:
  - `cargo check` — pass
  - `cargo test -q` — pass (0 tests)
- Issues / deviations:
  - None.
- Next steps:
  - Phase 1: terminal boundary + `App::run` + placeholder UI layout.

## Phase 1 (TUI Skeleton) Progress Update
- Work completed:
  - Implemented terminal raw-mode + alt-screen boundary with panic-safe restore (`src/terminal/mod.rs`).
  - Implemented `App::run` event loop with deterministic quit keys and placeholder rendering (`src/app/app.rs`, `src/ui/*`).
  - Implemented fail-fast fatal error printing after restoring terminal (`src/diagnostics/banner.rs`, `src/main.rs`).
  - Rendered sidebar with Channels + DMs + Threads sections (placeholder data) and a workspace canvas placeholder.
- Tests run + results:
  - `cargo check -q` — pass
  - `cargo test -q` — pass
  - `cargo run` — pass (TUI opens; `q` exits; terminal restored)
- Issues / deviations:
  - The TUI is not “pretty” yet; this is intentional until splits/keymap/composer land.
- Next steps:
  - Phase 2: split-tree SSOT + pane ops + keymap engine + composer widget.

## Phase 2 (Splits + Keymap) Progress Update
- Work completed:
  - Implemented `PaneTree` split-tree SSOT, pure pane ops (split/focus/close/resize), and a Ratatui layout renderer (`src/workspace/*`).
  - Wired workspace rendering so each pane is drawn with a focus highlight (`src/ui/workspace.rs`, `src/ui/root.rs`).
  - Added a default Vim-ish key dispatcher using `keybinds` (`Ctrl+w v/s/h/j/k/l/q`) and routed key handling through the input layer (`src/input/keymap.rs`, `src/input/normalize.rs`, `src/app/app.rs`).
  - Added fast workspace invariant tests (`tests/workspace_tree.rs`).
- Tests run + results:
  - `cargo check` — pass
  - `cargo test -q` — pass
- Issues / deviations:
  - `keybinds`’ optional crossterm integration pulled a second `crossterm` major version; removed that feature and added a small local `crossterm -> keybinds::KeyInput` conversion to keep a single crossterm in the dependency graph.
- Next steps:
  - Phase 2: add TOML config load/validate for keymaps.
  - Phase 2: replace composer placeholder with a real multiline composer (`tui-textarea`) and implement `Enter` send + `Shift+Enter` newline.
  - Phase 3: Slack auth/tokens + Socket Mode connection.

## Phase 2 (Splits + Keymap) Progress Update
- Work completed:
  - Added TOML config load (local `config.toml` first, then OS config dir via `directories::ProjectDirs`) and a `config.example.toml`.
  - Made keybindings config-driven with conflict detection and fail-loud startup validation (`src/input/keymap.rs`, `src/app/config.rs`, `src/app/app.rs`).
  - Loaded `.env` at startup for local dev (`dotenvy`), so `APP_TOKEN` + `BOT_TOKEN` can be sourced from `.env` without being part of the TOML config shape.
  - Added `SlackTokens::from_env()` with prefix validation and redacted debug output (`src/slack/tokens.rs`) for later Socket Mode/Web API wiring.
- Tests run + results:
  - `cargo fmt` — pass
  - `cargo check` — pass
  - `cargo test -q` — pass
- Issues / deviations:
  - None.
- Next steps:
  - Phase 2: implement a real multiline composer (`tui-textarea`) + deterministic key precedence (Normal vs Compose) so typing never triggers pane ops.
  - Phase 2: implement `Enter` send + `Shift+Enter` newline, and fail loudly if the terminal can’t distinguish it (require remap; no silent fallback).
  - Phase 3: add Slack deps + Socket Mode connect and populate sidebar from real Slack data.

## Phase 2 (Splits + Keymap) Progress Update
- Work completed:
  - Added Normal/Compose input modes so pane keybindings never conflict with typing (`src/input/keymap.rs`, `src/app/state.rs`, `src/app/app.rs`).
  - Replaced composer placeholder with a real `tui-textarea` widget (`src/ui/composer.rs`).
  - Implemented `Enter` send + `Shift+Enter` newline, and fail-fast gating to avoid the “Shift+Enter is indistinguishable so you accidentally send” footgun.
  - Enabled crossterm’s kitty keyboard protocol progressive enhancement flags to improve modified-key fidelity (`src/terminal/mod.rs`).
- Tests run + results:
  - `cargo fmt` — pass
  - `cargo check` — pass
  - `cargo test -q` — pass
- Issues / deviations:
  - None.
- Next steps:
  - Phase 3: add Slack deps + Socket Mode connect and populate sidebar from real Slack data (tokens will come from `.env` via `APP_TOKEN` / `BOT_TOKEN`, not TOML).

## Phase 3 (Slack Socket Mode Connect) Progress Update
- Work completed:
  - Wired Slack Socket Mode connection in a background Tokio runtime so the UI/render loop stays pure and never blocks on network I/O (`src/app/runtime.rs`, `src/app/app.rs`).
  - Added a minimal fail-fast Socket Mode websocket loop that ACKs envelopes immediately and exits on disconnect/errors (no reconnect) (`src/slack/socket_mode.rs`).
  - Defined a `SlackService` boundary for Slack API calls (starting with `apps.connections.open`) and kept slack-morphism types contained under `src/slack/*` (`src/slack/service.rs`, `src/slack/mod.rs`).
  - Pinned `serde_json` to `1.0.147` to avoid a toolchain mismatch in newer transitive deps (build break in `zmij`).
- Tests run + results:
  - `cargo fmt` — pass
  - `cargo check -q` — pass
  - `cargo test -q` — pass
- Issues / deviations:
  - Dependency/toolchain mismatch: newer `serde_json` pulled a `zmij` build that assumes `std::hint::select_unpredictable` exists for rustc 1.88+, but our toolchain does not have it; pinned versions instead of carrying RUSTFLAGS shims.
- Next steps:
  - Phase 3.1 (immediate): implement `--headless` scripted driver mode + a smoke integration test so agents/CI can run end-to-end checks without an interactive TTY.
  - Then Phase 3: populate sidebar from real Slack conversations (channels + DMs) and add selection/open-in-pane actions.

## Phase 3.1 (Headless Non-Interactive Driver) Progress Update
- Work completed:
  - Extracted the core action reducer (`apply_action`) so both interactive keybindings and headless scripts drive the same SSOT (`src/app/reducer.rs`, `src/app/app.rs`).
  - Added `--headless --script <path|->` mode that runs without a TTY and emits JSON lines for automation (`src/main.rs`, `src/app/headless.rs`).
  - Added a deterministic headless smoke integration test that spawns the binary and asserts output markers (`tests/headless_smoke.rs`).
  - Added a trap-check invariant so headless mode can never enter terminal raw mode / alt screen (`src/terminal/mod.rs`, `src/app/headless.rs`).
  - Added live Slack headless smoke commands:
    - `slack_auth_test` (Web API) emits a structured summary.
    - `slack_socket_mode_smoke <timeout_ms>` connects, waits for `hello`, ACKs envelopes, then exits (`src/slack/service.rs`, `src/slack/socket_mode.rs`, `src/app/headless.rs`).
  - Added an opt-in ignored integration test for live Slack connectivity (`tests/headless_live_slack.rs`).
- Tests run + results:
  - `cargo fmt` — pass
  - `cargo test -q` — pass
  - Manual: `cargo run -- --headless --script -` with `slack_auth_test` + `slack_socket_mode_smoke` — pass
- Issues / deviations:
  - Socket Mode initially failed with “TLS support not compiled in”; fixed by enabling `tokio-tungstenite` rustls TLS features (`rustls-tls-native-roots`).
- Next steps:
  - Phase 4: real sidebar data (channels/DMs/threads) + timeline panes + caching + real-time message updates (read path).

## Phase 3/4 (Real Sidebar + Timeline Read Path) Progress Update
- Work completed:
  - Added an effects boundary so Slack I/O is never executed on the render/input path (`src/app/effects.rs`, `src/app/app.rs`).
  - Populated the left sidebar with real Slack Channels + DMs (Threads section shell remains, per plan) and added Vim-ish defaults for navigation (`j/k`, `Enter`, `r`) (`src/ui/sidebar.rs`, `src/app/sidebar.rs`, `src/app/state.rs`).
  - Introduced minimal internal model types for conversations/messages/users and a per-conversation timeline cache (`src/model/*`, `src/app/timeline.rs`).
  - Added a `PaneKind::Timeline { conversation }` pane kind and wired “open selected conversation” to load recent history into the focused pane (`src/workspace/tree.rs`, `src/ui/workspace.rs`, `src/app/reducer.rs`, `src/slack/service.rs`).
  - Hardened the most common Slack read-path footgun: opening a channel the bot isn’t a member of fails fast with an actionable message (invite bot / use a token with access) rather than a vague UI state (`src/app/reducer.rs`).
  - Headless mode now runs effects to completion per script line, so Slack-backed reads are automatable without a TTY (`src/app/headless.rs`).
- Tests run + results:
  - `cargo test -q` — pass
  - Manual headless: `sidebar_refresh` — loads sidebar (requires tokens)
- Issues / deviations:
  - Slack `conversations.history` returns `not_in_channel` for channels the bot isn’t a member of; we now fail fast *before* the API call when opening from the sidebar so the error is actionable.
- Next steps:
  - Phase 4: cursor pagination (`P4.T3.1`) and real-time message event translation (`P4.T4`).
  - Phase 4: basic mrkdwn renderer (no raw JSON/debug placeholders).

## Phase 4 (Pagination + Real-Time Events) Progress Update
- Work completed:
  - Implemented cursor-based “load older” pagination for the focused timeline (`Action::TimelineLoadOlder`) and appended pages into the cache with `ts` dedup (`src/app/reducer.rs`, `src/app/effects.rs`, `src/slack/service.rs`).
  - Added a default Vim-ish binding for pagination (`Ctrl+u`) and exposed it in the config shape (`src/app/config.rs`, `src/input/keymap.rs`, `config.example.toml`).
  - Extended the headless JSON state output with timeline `has_more`/cursor visibility, and added an opt-in ignored live Slack pagination test (`src/app/headless.rs`, `tests/headless_live_slack_pagination.rs`).
  - Added Socket Mode envelope → action translation (`src/slack/events.rs`) and wired “new message” events into the reducer so loaded timelines update incrementally (no polling-first loop) (`src/slack/socket_mode.rs`, `src/app/reducer.rs`).
- Tests run + results:
  - `cargo test -q` — pass
  - (opt-in) `SLACK_LIVE_TEST=1 ... cargo test -q -- --ignored` — not run in this worklog entry
- Issues / deviations:
  - None.
- Next steps:
  - Phase 4: minimal mrkdwn renderer (`P4.T5`) so reading is useful beyond plain text snippets.
  - Phase 4 cleanup: remove any remaining debug/placeholder rendering paths (`P4.T6`).

## Phase 4 (Minimal Renderer) Progress Update
- Work completed:
  - Added a minimal Slack mrkdwn → plaintext renderer (mentions + links + basic entity unescape) and used it for timeline pane rendering (`src/render/mrkdwn.rs`, `src/ui/workspace.rs`).
  - Added small renderer unit tests so we don’t regress mention/link handling while iterating quickly (`tests/mrkdwn_render.rs`).
- Tests run + results:
  - `cargo test -q` — pass
- Issues / deviations:
  - Renderer is intentionally conservative (unknown tokens are preserved; no full Markdown parsing).
- Next steps:
  - Phase 4 cleanup (`P4.T6`): ensure the UI has a single consistent rendering path (no ad-hoc “snippet” fallbacks).
  - Phase 5: threads sidebar section + per-pane message selection model (so thread/reaction actions have a target).

## Phase 3.1/4 (Headless Real-Time + Fail-Fast) Progress Update
- Work completed:
  - Added a bounded headless Socket Mode listen command (`slack_socket_mode_listen <duration_ms>`) that drains live events into `Action` and applies them to state (`src/slack/socket_mode.rs`, `src/app/headless.rs`, `tests/headless_live_slack_socket_mode_listen.rs`).
  - Enforced explicit Slack effect timeouts in both interactive and headless paths so automation can’t hang indefinitely (`src/app/effects.rs`, `src/app/app.rs`, `src/app/headless.rs`).
  - Made cursor presence the only pagination “has more” SSOT (removed `TimelineState.has_more`; derived from `next_cursor`) (`src/app/timeline.rs`, `src/app/reducer.rs`, `src/slack/service.rs`, `tests/timeline_pagination.rs`).
  - Made Socket Mode status events fail-fast instead of being silently dropped if the UI channel is full (`src/slack/socket_mode.rs`).
- Tests run + results:
  - `cargo fmt` — pass
  - `cargo test -q` — pass
- Issues / deviations:
  - None.
- Next steps:
  - Phase 5: implement the Slack write path (`chat.postMessage` + headless coverage) so we can deterministically validate “send → Socket Mode event → timeline update” end-to-end without a human.

## Phase 5 (Send Pipeline — Channel/DM) Progress Update
- Work completed:
  - Implemented a fail-fast Slack send effect (`chat.postMessage`) and wired `ComposerSend` to emit it based on the focused timeline pane (`src/app/reducer.rs`, `src/app/effects.rs`, `src/slack/service.rs`).
  - Added a headless `type <text>` command (with `\\n` escapes) so composer + send can be driven non-interactively (`src/app/headless.rs`).
  - Added an opt-in ignored live Slack integration test that posts a unique message and asserts the focused timeline message count increments by 1 (explicitly gated behind `SLACK_LIVE_TEST_ALLOW_WRITES=1`) (`tests/headless_live_slack_send_path.rs`).
- Tests run + results:
  - `cargo fmt` — pass
  - `cargo test -q` — pass
- Issues / deviations:
  - None.
- Next steps:
  - Threads: add a thread pane and plumb `thread_ts` into the send effect for thread replies.
  - Scroll/selection: add a per-timeline cursor + scroll offset so pagination is actually usable.

## Phase 5 (Timeline Selection + Focus Model) Progress Update
- Work completed:
  - Added a focus-area toggle (sidebar vs workspace) so Vim-ish keys can mean “sidebar navigation” vs “timeline selection” deterministically (`src/input/keymap.rs`, `src/app/state.rs`, `src/ui/root.rs`).
  - Implemented a per-pane selected-message cursor (`selected_ts`) and initialized it on load so every timeline pane has an explicit target for thread/reaction actions (`src/app/pane_view.rs`, `src/app/reducer.rs`, `src/workspace/tree.rs`).
  - Switched timeline rendering to a stateful list that highlights the selected message and scrolls as selection moves (`src/ui/workspace.rs`).
  - Exposed selection state in headless JSON output (`focused_timeline_selected_ts`) and added a small unit test to lock selection ordering (`src/app/headless.rs`, `tests/timeline_selection.rs`).
- Tests run + results:
  - `cargo fmt --all` — pass
  - `cargo test -q` — pass
- Issues / deviations:
  - None.
- Next steps:
  - Threads: implement `PaneKind::Thread`, fetch replies via `conversations.replies`, populate the sidebar Threads list (recent/open in this client), and enable thread reply sends.

## Phase 5 (Threads + Thread Replies) Progress Update
- Work completed:
  - Added a minimal thread model (`ThreadKey`) and sidebar Threads list state that remains visible alongside Channels + DMs (`src/model/thread.rs`, `src/app/sidebar.rs`, `src/ui/sidebar.rs`).
  - Added `PaneKind::Thread` and actions/effects to open a thread from the selected message and fetch replies via `conversations.replies` (`src/workspace/tree.rs`, `src/app/action.rs`, `src/app/reducer.rs`, `src/app/effects.rs`, `src/slack/service.rs`, `src/ui/workspace.rs`).
  - Wired workspace keybind `open_thread` (default `Enter` when workspace-focused) and headless actions/metrics for threads (`src/input/keymap.rs`, `config.example.toml`, `src/app/headless.rs`).
  - Enabled thread reply sends via `thread_ts` (focused thread pane → `chat.postMessage(thread_ts=...)`) and added an opt-in ignored live headless test (`tests/headless_live_slack_thread_reply.rs`).
- Tests run + results:
  - `cargo fmt --all` — pass
  - `cargo test -q` — pass
- Issues / deviations:
  - Thread sidebar titles are intentionally minimal right now (`<conversation> — <thread_ts>`); we can improve them later using the root message snippet.
- Next steps:
  - Reactions: add/remove via `reactions.add` / `reactions.remove` using the selected message cursor as the target.

## Phase 5 (Reactions) Progress Update
- Work completed:
  - Extended the internal message model to include reaction counts and “me” flag so the UI can render reactions deterministically (`src/model/message.rs`, `src/slack/service.rs`).
  - Added a minimal status-line prompt for emoji name and wired `r` (configurable) to toggle add/remove via `reactions.add` / `reactions.remove` (`src/app/prompt.rs`, `src/app/app.rs`, `src/app/reducer.rs`, `src/app/effects.rs`).
  - Added headless `react <emoji>` so automation can test reactions without prompt UX (`src/app/headless.rs`).
  - Added an opt-in ignored live Slack test for reaction toggle (`tests/headless_live_slack_reaction_toggle.rs`).
- Tests run + results:
  - `cargo fmt --all` — pass
  - `cargo test -q` — pass
- Issues / deviations:
  - Socket Mode reaction events are not handled yet; reaction state updates on successful Web API calls.
- Next steps:
  - Files + snippets: modern upload flow + authenticated downloads + usable rendering in timelines.

## Phase 6 (Files + Snippets + Block Kit) Progress Update
- Work completed:
  - Extended internal message model to include attached files (`src/model/message.rs`, `src/slack/service.rs`, `src/slack/events.rs`).
  - Implemented modern external upload flow (`files.getUploadURLExternal` → upload bytes → `files.completeUploadExternal`) (`src/slack/files.rs`, `src/slack/service.rs`, `src/app/effects.rs`, `src/app/reducer.rs`).
  - Implemented authenticated downloads from `url_private_download` with atomic writes (`.part` temp + rename) (`src/slack/files.rs`, `src/app/effects.rs`).
  - Added file upload/download prompts + keybinds (`f u` / `f d` by default; configurable) and headless `upload` / `download` commands (`src/app/prompt.rs`, `src/app/app.rs`, `src/input/keymap.rs`, `src/app/headless.rs`, `config.example.toml`).
  - Added best-effort snippet preview via `files.info` (lazy: triggered from selection) (`src/slack/files.rs`, `src/app/reducer.rs`).
  - Rendered file attachments/snippets in timeline + thread panes (`src/render/attachments.rs`, `src/ui/workspace.rs`).
  - Added minimal Block Kit plaintext rendering for `section`/`context`/`divider` (blocks-first) (`src/render/blocks.rs`, `src/slack/service.rs`).
  - Added an opt-in ignored live Slack test that uploads then downloads a file and asserts bytes match (`tests/headless_live_slack_file_upload_download.rs`).
- Tests run + results:
  - `cargo fmt --all` — pass
  - `cargo test -q` — pass
- Issues / deviations:
  - File metadata completeness varies by API surface; size/preview are filled lazily via `files.info` when needed.
- Next steps:
  - Phase 7: write minimal README/setup instructions and do the final acceptance checklist pass.

## Phase 7 (Docs) Progress Update
- Work completed:
  - Added a README with setup instructions, token sourcing (`.env` APP_TOKEN/BOT_TOKEN), keybind config notes, and headless usage (`README.md`, `config.example.toml`).
- Tests run + results:
  - `cargo test -q` — pass
- Issues / deviations:
  - None.
- Next steps:
  - Run the Section 0.4 acceptance checklist in your real workspace and fix only what blocks daily usage.

## Phase 7 (Stability Pass) Progress Update
- Work completed:
  - Ran `cargo clippy -q -- -D warnings` and fixed minor clippy findings (prompt input else-if collapse, Default init cleanup, iterator loop cleanup, small UI string + index hygiene).
- Tests run + results:
  - `cargo fmt --all` — pass
  - `cargo test -q` — pass
  - `cargo clippy -q -- -D warnings` — pass
- Issues / deviations:
  - None.
- Next steps:
  - Cut the initial git commit on `feat/tui-skeleton` and proceed to manual acceptance (non-blocking per plan).

## Phase 2 (Workspace Resize Wiring) Progress Update
- Work completed:
  - Wired pane resize through `Action` + config + keymap + reducer (so it works in both interactive TUI and headless scripts) (`src/app/action.rs`, `src/app/config.rs`, `src/input/keymap.rs`, `src/app/reducer.rs`, `config.example.toml`).
  - Added a deterministic test that asserts resize changes pane geometry via the layout renderer (`tests/workspace_resize.rs`).
- Tests run + results:
  - `cargo fmt --all` — pass
  - `cargo test -q` — pass
  - `cargo clippy -q -- -D warnings` — pass
- Issues / deviations:
  - None.
- Next steps:
  - Commit + push the resize wiring fix; then proceed with the manual acceptance checklist (non-blocking per plan).
