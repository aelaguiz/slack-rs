---
title: "slack-rs — Terminal-Driven Slack TUI — Architecture Plan"
date: 2026-03-01
status: active
fallback_policy: forbidden
owners: [aelaguiz]
reviewers: [aelaguiz]
doc_type: new_system
related:
  - https://api.slack.com/
  - https://github.com/ratatui-org/ratatui
---

# TL;DR

- **Outcome:** A terminal-first Slack client written in Rust with Vim-style, configurable keybindings and an arbitrarily splittable workspace so channels/DMs/threads can be viewed side-by-side.
- **Problem:** The native Slack clients aren’t terminal-native and don’t support a Vim-first, multi-pane workflow for quickly navigating channels/DMs/threads and acting on messages (reactions, files, snippets) without leaving the keyboard.
- **Approach:** Build a Rust TUI with (1) a left “Slack bar” navigator, (2) a right-side split-pane workspace, (3) a keybinding/config system inspired by Vim, and (4) a Slack integration layer that supports channels, DMs, threads, emoji reactions, file up/download, and snippet rendering.
- **Plan:** Phase 1: TUI skeleton + layout tree + keymap config; Phase 2: Slack auth + sidebar lists + message timeline panes; Phase 3: threads + reactions + composing; Phase 4: files + snippet rendering + polish; Phase 5: packaging + docs + stability pass.
- **Non-negotiables:**
  - Keyboard-only, terminal-first UX (no mouse required).
  - Configurable Vim-style keybindings (deterministic, conflict-free).
  - Right-side workspace supports arbitrary splits, focus movement, resize, and close via keybindings.
  - Near real-time message updates (no manual refresh loop as the primary UX).
  - Non-interactive testability: the app can run headless, accept scripted commands, and emit machine-readable logs/output so we can run end-to-end checks without a human driving a terminal.
  - Slack parity for core workflows: channels, DMs, threads, reactions, file upload/download, snippet rendering.
  - Fail-fast boundaries: crash with an explicit error chain (and backtrace when available); do not auto-recover or paper over bugs.
  - Token/credential safety: never log secrets; store securely or require explicit user opt-in.
  - Execution posture: default to “implement and iterate” — do not block on open questions or “nice-to-haves”; only stop for true blockers (missing access/credentials, contradictory scope, or a violated invariant).

Worklog: `docs/TERMINAL_DRIVEN_RUST_SLACK_TUI_WITH_VIM_KEYBINDINGS_2026-03-01_WORKLOG.md`

---

<!-- arch_skill:block:implementation_audit:start -->
# Implementation Audit (authoritative)
Date: 2026-03-02
Verdict (code): COMPLETE
Manual QA: pending (non-blocking)

## Code blockers (why code isn’t done)
- None known.

## Reopened phases (false-complete fixes)
- None.

## Missing items (code gaps; evidence-anchored; no tables)
- None known.

## Non-blocking follow-ups (manual QA / screenshots / human verification)
- Manual: open a known channel/DM, send a message from another Slack client, confirm it appears in `slack-rs` within seconds (Socket Mode → reducer).
- Manual: use `Ctrl+u` to paginate older history several times and confirm message count increases until no further cursor/older pages remain.
- Manual: `Tab` to focus the workspace and use `j/k` (and `gg/G`) to move the selected message highlight in the focused timeline pane.
- Manual: split into 2 panes, then use the resize bindings to confirm the focused pane grows/shrinks (defaults in `config.example.toml`).

## External second opinions
- Opus: received
  - Key points:
    - Verified: pane resize is reachable via Action → config → keymap dispatch → reducer (`src/app/action.rs`, `src/app/config.rs`, `src/input/keymap.rs`, `src/app/reducer.rs`).
    - Verified: programmatic test proves resize changes geometry via deterministic layout rects (`tests/workspace_resize.rs`).
  - Disposition: accepted — wiring is complete and idiomatic.
- Gemini: received
  - Key points:
    - Verified: resize is wired end-to-end (actions/config/keymap/reducer) and usable from both sidebar/workspace focus maps (`src/input/keymap.rs`).
    - Verified: tests assert pane width/height changes, preventing silent no-ops (`tests/workspace_resize.rs`).
  - Disposition: accepted — implementation matches the plan.
<!-- arch_skill:block:implementation_audit:end -->

---

<!-- arch_skill:block:planning_passes:start -->
<!--
arch_skill:planning_passes
deep_dive_pass_1: done 2026-03-01
external_research_grounding: done 2026-03-01
deep_dive_pass_2: done 2026-03-01
recommended_flow: deep dive -> external research grounding -> deep dive again -> phase plan -> implement
note: This is a warn-first checklist only. It should not hard-block execution.
-->
<!-- arch_skill:block:planning_passes:end -->

---

# 0) Holistic North Star

## 0.1 The claim (falsifiable)
> If we build `slack-rs` as a Rust terminal UI Slack client with Vim-style keybindings, an arbitrarily splittable workspace, and a sidebar that always shows Channels + DMs + Threads (recent/open), then I can do my day-to-day Slack work entirely from the terminal (channels, DMs, threads, emoji reactions, file upload/download, snippet rendering), with near real-time updates and scrollback that can reach the beginning of a conversation (lazy-loaded via pagination), measured by (1) completing a keyboard-only workflow checklist in a real Slack workspace and (2) running a deterministic **non-interactive headless smoke script** that drives actions and captures logs/output — while the app fails fast with a clear error chain (no silent recovery, no auto-reconnect) whenever something breaks — by the time we tag `v0.1.0`.

## 0.2 In scope
- UX surfaces (what users will see change):
  - A terminal UI with:
    - Left sidebar (“Slack bar”) for **Channels + DMs + Threads** (always visible; opening threads must not “replace” the channels list).
      - Thread list semantics: “recent/open threads in this client” is sufficient (not a global “all my threads” view).
    - Right workspace that can split into multiple panes (including nested splits).
  - Pane types to view side-by-side:
    - Channel timeline view
    - DM timeline view
    - Thread view (replies/context)
  - Core interactions:
    - Sidebar navigation across channels, DMs, and threads (thread list view + open thread).
    - Message composition + send
    - Thread reply
    - Emoji reactions (add/remove)
    - File upload and download
    - Timeline pagination to reach full history (load older messages on demand; do not hard-cap scrollback)
    - Rendering of Slack text + code blocks + snippets/attachments at a “usable by default” level
  - Vim-style, configurable keybindings for navigation, pane management, and message actions.
- Technical scope (what code will change):
  - Rust workspace for a terminal app (TUI rendering, input loop, async runtime).
  - Layout/splitting engine (pane tree, focus model, resizing/closing).
  - Keybinding/config layer (Vim-inspired modes or leader-based commands; conflict detection; deterministic precedence rules).
  - Slack API integration (auth, list channels/DMs, fetch/post messages, threads, reactions, files).
  - Non-interactive driver mode (headless): run the app without a real TTY, accept scripted commands, and emit machine-readable logs/output for automation.
  - State management + caching (local model of sidebar + open panes; message caches sized/capped to avoid runaway memory).
  - Error handling, logging, and minimal observability suitable for a terminal app.

## 0.3 Out of scope
- UX surfaces (what users must NOT see change):
  - Any non-terminal UI (no GUI desktop client, no mobile app, no web app).
  - Slack voice/video calls, huddles, canvases, workflows, or admin/enterprise management features.
  - “Full fidelity” rendering of every Slack attachment type on day one (we’ll support the core set required for usability and expand intentionally).
  - A “which-key”/cheatsheet UI (defer; add only if we explicitly decide it’s needed later).
  - A key-event diagnostics screen (defer; add only if terminal input issues block progress).
  - A dedicated notes/scratchpad feature (this is Slack, not a note-taking app).
  - Sidebar search/jump UI (defer; add only if we miss it).
- Technical scope (explicit exclusions):
  - Runtime shims/fallback paths that silently change behavior when something isn’t implemented.
  - Automatic recovery/reconnect behavior that masks problems (during `v0.1.0` we prefer fail-fast + explicit error).
  - A plugin system or scripting engine (unless later added as a deliberate phase).
  - Cross-device sync beyond what Slack already provides via its APIs.

## 0.4 Definition of done (acceptance evidence)
- The app runs in a terminal and is usable with keyboard-only navigation.
- The app is testable in non-interactive mode:
  - It can start without a human-operated terminal (no TTY required).
  - A script can drive high-level commands/actions (at minimum: split panes, move focus, enter/leave compose, quit).
  - Logs/output can be captured programmatically (stdout/stderr) for end-to-end verification.
- Authentication is possible for a real Slack workspace, and the app can load channel + DM lists.
- Left sidebar supports navigating channels, DMs, and threads with Vim-style keybindings (channels remain visible even when threads are open in panes).
- Right workspace can:
  - Split/close/resize panes via keybindings.
  - Show a channel timeline and a thread view side-by-side.
  - Show a channel and a DM side-by-side.
- Core Slack workflows work end-to-end:
  - Read messages in channels and DMs.
  - Load older messages (pagination) until you can reach the beginning of a conversation (lazy-loaded on demand; not necessarily fetched all at once).
  - Post a message in a channel and a DM.
  - Reply in a thread and view the thread context.
  - Add/remove emoji reactions on a message.
  - Upload a file to a conversation and download a file locally.
  - Render message formatting and snippets/attachments in a way that supports real work (not just raw JSON).
  - When something goes wrong (Slack API error, Socket Mode error, parsing/model error), the app fails fast with an explicit error chain and restores the terminal to normal mode.
- Evidence plan (common-sense; non-blocking):
  - Primary signal (keep it minimal; prefer existing tests/checks): Manual keyboard-only workflow checklist (items above) in a real Slack workspace — all steps pass without mouse use or crashes.
  - Optional second signal (only if needed): A deterministic headless smoke script (`--headless`/driver mode) that runs a known command sequence and asserts expected output/log markers — runnable by an agent/CI without an interactive TTY.
  - Optional third signal (only if needed): Basic structured logs for Slack API failures/rate limits — no unhandled errors during the checklist; failures are surfaced as an explicit fatal error chain on exit (and/or logs), not silently swallowed.
  - Default: do NOT add bespoke screenshot harnesses / drift scripts unless they already exist in-repo or are explicitly requested.
  - Avoid negative-value tests/gates: do NOT add “deleted code not referenced” tests, visual-constant tests (colors/margins/pixels), doc-driven inventory gates, or mock-only interaction tests.
- Metrics / thresholds (if relevant):
  - Crash-free checklist run: 0 panics during a full checklist pass.
  - Input responsiveness: no dropped keystrokes during navigation/pane operations in a typical workspace.

## 0.5 Key invariants (fix immediately if violated)
- No fallbacks: unsupported Slack behaviors must fail loudly with a clear message (and ideally a “not implemented yet” explanation), not silently degrade.
- No masked bugs: prefer fail-fast over “keep running” recovery. If a correctness-affecting error occurs, restore the terminal and exit/crash with a clear error chain (and backtrace when available).
- No auto-recovery: do not auto-reconnect, “retry forever”, or attempt to keep operating on stale state after Slack I/O failures; crash loudly so the bug can be fixed quickly.
- UI event loop must never be blocked by network I/O; all Slack calls happen off the render/input path.
- Single source of truth for app state (sidebar state + open panes + focus + selections); no duplicated “shadow state” per-pane that drifts.
- Keybinding interpretation is deterministic and conflict-free (no ambiguous mappings without an explicit resolver rule).
- Non-interactive testability is always maintained: core actions must be runnable without a TTY (via a headless driver), and logs/output must be capturable programmatically.
- Secrets safety: tokens are never logged; error messages are scrubbed; on-disk storage is explicit and auditable.
- Fallback policy (strict):
  - Default: **NO fallbacks or runtime shims** (feature must work correctly or fail loudly).
  - If an exception is truly required, it must be explicitly approved by aelaguiz by setting `fallback_policy: approved` and recording a Decision Log entry with a timebox + removal plan.

---

# 1) Key Design Considerations (what matters most)

## 1.1 Priorities (ranked)
1) <#1>
2) <#2>
3) <#3>

## 1.2 Constraints
- Correctness:
- Performance:
- Offline / latency:
- Compatibility / migration (default: hard cutover; no shims):
- Operational / observability:

## 1.3 Architectural principles (rules we will enforce)
- <e.g., fail-loud boundaries, DI rules, no business logic in UI, etc.>
- Pattern propagation via comments (high leverage; no spam):
  - When we introduce a new SSOT/contract or a non-obvious “gotcha”, add a short doc comment in the canonical boundary module explaining the invariant + how to extend it safely.
  - Do NOT comment everything; comment the tricky bits we want to propagate forward.

## 1.4 Known tradeoffs (explicit)
- <tradeoff> → chosen direction + why
- Alternatives rejected + why

---

# 2) Problem Statement (existing architecture + why change)

## 2.1 What exists today
- <system overview in 5–10 bullets>
- Primary flows / control paths:
  - <flow A>
  - <flow B>

## 2.2 What’s broken / missing (concrete)
- Symptoms:
- Root causes (hypotheses):
- Why now:

## 2.3 Constraints implied by the problem
- <constraints derived from reality, not preference>

---

# 3) Research Grounding (external + internal “ground truth”)

<!-- arch_skill:block:research_grounding:start -->
## External anchors (papers, systems, prior art)
- Slack Socket Mode docs — https://api.slack.com/apis/connections/socket — **Adopt:** Socket Mode as the default real-time feed (Events API over WebSocket) — avoids polling-first designs and doesn’t require a public HTTP endpoint.
- Slack Events API overview — https://api.slack.com/apis/connections/events-api — **Adopt:** treat events as the primary “incremental updater”; use Web API calls for initial sync + reconciliation only — keeps the UI responsive and rate-limit aware.
- Slack rate limits overview + 2025-05 rate limit update/FAQ — https://api.slack.com/docs/rate-limits and https://api.slack.com/changelog/2025-05-terms-rate-limit-update-and-faq — **Adopt:** caching, incremental updates, and conservative use of `conversations.history`/`conversations.replies` (with backoff) — this is a personal/internal tool (non-Marketplace posture), so we should assume stricter limits.
- Slack “Uploading files” guide + 2024-04 changelog about “better uploads” — https://api.slack.com/messaging/files/uploading and https://docs.slack.dev/changelog/2024-04-a-better-way-to-upload-files-is-here-to-stay — **Adopt:** modern 3-step upload (`files.getUploadURLExternal` → upload bytes → `files.completeUploadExternal`) — `files.upload` is retired and not a safe foundation.
- Slack formatting message text (mrkdwn) — https://docs.slack.dev/messaging/formatting-message-text/ — **Adopt:** implement Slack-specific tokens and escaping rules (mentions, channels, links, code blocks) explicitly — Slack mrkdwn is Markdown-like but not CommonMark.
- Slack Block Kit + message composition guidance — https://api.slack.com/block-kit and https://api.slack.com/methods/chat.postMessage — **Adopt:** render `blocks` first when present and include `text` as fallback on send — aligns with modern Slack message structure.
- Ratatui + backends docs — https://github.com/ratatui/ratatui and https://ratatui.rs/concepts/backends/ — **Adopt:** `ratatui` + `crossterm` as the UI stack — immediate-mode layout primitives are a good fit for an app-owned split-pane workspace.
- Crossterm event docs — https://docs.rs/crossterm/latest/crossterm/event/index.html — **Adopt:** single event loop + keyboard enhancement flags (when available) — keybinding fidelity (including modified keys like `Shift+Enter`) depends on terminal reporting.
- keybinds crate docs — https://docs.rs/keybinds — **Adopt:** configurable key sequences + mode-aware dispatch for app-level actions — matches “Vim-style, configurable keybindings”.
- tui-textarea crate docs — https://docs.rs/tui-textarea — **Adopt:** multiline composer widget — avoids reinventing cursor/scroll/edit history for the most important text field in the app.

## Internal ground truth (code as spec)
- Authoritative behavior anchors (do not reinvent):
  - `docs/TERMINAL_DRIVEN_RUST_SLACK_TUI_WITH_VIM_KEYBINDINGS_2026-03-01.md` — North Star, scope, invariants, target UI wireframes, and keybinding semantics — evidence: Sections 0, 5.5, and Decision Log entries.
- Existing patterns to reuse:
  - (None — repo is currently documentation-only. Once code exists, this section becomes the “reuse map” for modules, patterns, and test fixtures.)

## Open questions (evidence-based)
- These are explicitly **non-blocking**: we keep building, and we settle them with the smallest-possible spike/observation when they matter.
- Can we reliably distinguish `Shift+Enter` from `Enter` in the target terminal(s) using `crossterm` + enhanced keyboard reporting? — evidence: a minimal key-event dumper / diagnostics screen run in the terminal(s) you actually use (kitty/wezterm/iTerm2/etc.).
- Auth UX for a personal tool: start with manual token entry vs implement OAuth v2 (PKCE + loopback redirect)? — evidence: spike the minimal auth path and confirm required scopes for threads/files in your workspace.
- Minimal event set for “usable real-time”: which Events API events must we subscribe to for messages, edits/deletes, reactions, and thread replies? — evidence: Slack Events docs + a tiny Socket Mode prototype; verify behavior against actual message activity.
- Rendering scope for Block Kit: which block types show up frequently enough in your workspace to be “day-one support”? — evidence: capture a handful of real message payloads and classify blocks/attachments/files.
<!-- arch_skill:block:research_grounding:end -->

---

<!-- arch_skill:block:external_research:start -->
# External Research (best-in-class references; plan-adjacent)

> Goal: anchor the plan in idiomatic, broadly-accepted practices where applicable. This section intentionally avoids project-specific internals.

## Topics researched (and why)
- Rust TUI frameworks + terminal backends — to accelerate building a Slack-like “sidebar + workspace panes” UI with nested splits.
- Keybinding + input + composer widgets — because the product is keyboard-only and Vim-keybinding driven, and composition UX is critical.
- Async runtime + cancellation patterns — because the UI must stay responsive while Slack I/O and real-time events run concurrently.
- Slack API integration + real-time event strategy — to avoid “poll everything” designs and to align with modern Slack APIs (Socket Mode, Blocks).
- Files + message rendering — because uploads/downloads/snippets and readable formatting are core parity items, and Slack’s file APIs changed recently.
- Automation testing (headless + PTY-driven E2E) — because this project must be runnable/testable non-interactively end-to-end (including live Slack connectivity) by agents/CI.

## Findings + how we apply them

### Rust TUI frameworks + terminal backends
- Best practices (synthesized):
  - Prefer “immediate mode UI” (render from state each frame) when you need an arbitrarily reconfigurable, nested split workspace; model the split layout as data (a split tree) and render deterministically from it.
  - Treat “pane management” (split/close/resize/focus routing) as an app-level system: most Rust TUI crates give layout primitives, not a full tiling window manager.
  - Pick one terminal backend and keep it consistent across the dependency graph (especially if using `crossterm`) to avoid subtle raw-mode and input bugs.
  - Evaluate frameworks based on: widget ecosystem (lists/tables/scroll/paragraph), Unicode handling, input support, and community health (recent releases + docs).
  - Options worth considering:
    - `ratatui` (recommended default): strong widget ecosystem, flexible layout composition, actively maintained.
    - `tui-realm`: a component framework on top of Ratatui (props/state/message loop) that can reduce focus/event boilerplate for complex UIs, at the cost of an added abstraction layer.
    - `cursive`: a retained-mode view tree with many batteries-included widgets; great for quick “screens”, less directly aligned with an interactive tiling workspace.
    - `iocraft`: a newer, declarative “React-like” TUI with flexbox layout; promising for composition, but smaller widget ecosystem than Ratatui.
    - Avoid: `tui-rs` (archived) as a foundation for new work; treat old examples as conceptual references only.
  - Community health snapshot (as of this doc date, 2026-03-01):
    - Ratatui is actively releasing (GitHub shows a latest release in Dec 2025).
    - tui-realm is actively releasing (GitHub shows a latest release in Dec 2025).
- Recommended default for this plan:
  - Use `ratatui` + `crossterm` as the baseline UI stack, and implement our own split-tree workspace + focus routing (because it’s a core product feature and we’ll want full control).
  - Keep `tui-realm` as an optional upgrade path if focus/event wiring becomes the bottleneck, but do not start there unless we explicitly want its architecture.
- Pitfalls / footguns:
  - Multiple `crossterm` major versions pulled into the same binary can lead to confusing event/raw-mode behavior; avoid this by pinning and auditing features.
  - Terminal portability: `Alt`/`Option` and “enhanced keyboard reporting” vary by terminal; treat keybinding reliability as a feature with diagnostics, not an assumption.
- Sources:
  - Ratatui (GitHub) — https://github.com/ratatui/ratatui — Canonical repo + releases; community health signal.
  - Ratatui backends guide — https://ratatui.rs/concepts/backends/ — Backend tradeoffs + explicit warning about multiple `crossterm` versions.
  - Crossterm event docs — https://docs.rs/crossterm/latest/crossterm/event/index.html — Raw mode, event model, and keyboard enhancement flags.
  - tui-rs (archived) — https://github.com/fdehau/tui-rs — Shows archived/read-only status; explains why Ratatui is the modern continuation.
  - tui-realm (GitHub) — https://github.com/veeso/tui-realm — Higher-level component framework option for Ratatui apps.
  - Cursive (GitHub) — https://github.com/gyscos/cursive — Retained-mode alternative with mature widget set.
  - iocraft (GitHub) — https://github.com/ccbrown/iocraft — Declarative, component-based TUI alternative (smaller ecosystem; evaluate via spike).

### Keybindings, focus routing, and message composition widgets
- Best practices (synthesized):
  - Separate layers:
    - Terminal input capture (backend events) → normalized key events
    - Keymap/mode engine (Vim-like sequences, leader, timeouts) → high-level `Action`s
    - App reducer/state machine (pure state updates) → render
  - Treat focus as a first-class system: in a multi-pane UI, you need explicit “who receives keys” rules (focused widget first; global keymap next; never both).
  - Plan for terminal protocol realities:
    - Some key chords are indistinguishable without enhanced keyboard protocols (e.g., `Tab` vs `Ctrl-i`); build a “keybinding diagnostics” screen and document terminal expectations.
    - Modified keys like `Shift+Enter` are terminal-dependent; prefer enabling enhanced keyboard reporting (kitty protocol where available) and keep composer send/newline bindings configurable.
  - Reuse existing widgets for text entry:
    - A Slack client lives and dies on the composer; use a battle-tested textarea/editor widget instead of rolling your own cursor/selection/undo.
  - Libraries that accelerate this area:
    - `tui-textarea`: multiline editor widget with a Vim-like example; good default composer building block.
    - `edtui`: more Vim-inspired editing out of the box; consider if we want deeper Vim motions/commands in the composer.
    - `keybinds`: configurable key sequences and dispatch with a built-in timeout (prefix/leader-friendly).
    - `crokey`: strong for parsing/formatting key chords and generating help text; best paired with a sequence engine.
    - `rat-focus`: focus routing utilities for Ratatui UIs (useful when there are multiple interactive widgets).
    - `tui-input`: good for single-line inputs (search/jump dialogs).
- Recommended default for this plan:
  - Use `keybinds` for app-level key sequences + mode dispatch (Normal/Global), and use `tui-textarea` as the message composer (with a minimal modal layer to align with Vim-style UX).
  - Composer UX decision: `Enter` sends; newline uses `Shift+Enter` (not all terminals distinguish this — make it configurable, and offer an alternate default like `Ctrl+Enter` or `Alt+Enter` if needed).
  - Enable keyboard enhancement flags via `crossterm` when supported (best-effort), but **do not** build help/diagnostics UI up-front; keep bindings configurable and add a diagnostics surface only if terminal quirks block progress.
  - Evaluate `edtui` only if we truly want “Vim editor inside the composer” (motions/visual mode); otherwise keep the composer simpler.
- Pitfalls / footguns:
  - Letting the global keymap “steal” keys from the focused editor widget leads to maddening UX; enforce key routing precedence.
  - macOS terminal `Alt` behavior is inconsistent; expect to document “Option-as-Alt” requirements and provide default bindings that work even without Alt.
- Sources:
  - keybinds (crate docs) — https://docs.rs/keybinds — Configurable multi-key sequences + dispatch (explicitly “Vim style”) and timeouts.
  - crokey (crate docs) — https://docs.rs/crokey/latest/crokey/ — Key chord parsing/formatting + kitty-aware combiner; great for help overlays.
  - tui-textarea (crate docs) — https://docs.rs/tui-textarea — Multiline editor widget with a Vim-like example and customization points.
  - edtui (crate docs) — https://docs.rs/edtui — Vim-inspired editor widget option for Ratatui.
  - rat-focus (crate docs) — https://docs.rs/crate/rat-focus/2.0.1 — Focus routing for multiple widgets in Ratatui apps.
  - Crossterm keyboard enhancement flags — https://docs.rs/crossterm/latest/crossterm/event/index.html — Primary reference for enabling enhanced keyboard reporting.
  - Crossterm `KeyboardEnhancementFlags` — https://docs.rs/crossterm/latest/crossterm/event/struct.KeyboardEnhancementFlags.html — Details on kitty keyboard protocol progressive enhancement and key disambiguation.
  - Kitty keyboard protocol — https://sw.kovidgoyal.net/kitty/keyboard-protocol/ — Canonical spec for reporting modifiers and disambiguating keys in terminals.
  - Yazi keymap docs (real-world TUI keybinding UX) — https://yazi-rs.github.io/docs/configuration/keymap/ — Practical notation + terminal caveats (CSI-u, Alt on macOS).

### Async runtime + cancellation patterns (Tokio) for non-blocking TUIs
- Best practices (synthesized):
  - Use one “front door” async loop that multiplexes:
    - terminal input events (keypress/resize)
    - Slack Socket Mode events
    - periodic ticks (redraw, retry/backoff, timeouts)
    - shutdown signals
  - Prefer `tokio::select!` for multiplexing, but be explicit about cancellation behavior:
    - `select!` drops (cancels) non-winning futures; avoid putting non-cancellation-safe operations inside a looped `select!`.
  - Use explicit cancellation for long-running tasks:
    - a cancellation token for shutdown
    - per-task cancellation for file transfers and any long-lived Socket Mode/background tasks
  - Keep state transitions pure:
    - reducer mutates `AppState` only
    - async tasks emit `Action`s back into the reducer via a channel
- Recommended default for this plan:
  - Use `tokio::select!` to drive the main app loop, and use `tokio_util::sync::CancellationToken` to coordinate graceful shutdown of background tasks.
  - Use `tokio::sync::mpsc` (or `broadcast` where appropriate) to feed `Action`s into the reducer/effects boundary.
- Pitfalls / footguns:
  - Putting non-cancellation-safe operations inside a looped `select!` can cause “lost progress” when a branch gets dropped and restarted repeatedly.
  - Spawning tasks that outlive the UI without a cancellation story (surprising Slack calls after quit; resource leaks).
  - Holding locks across `.await` (especially inside select loops) — easy deadlock / starvation trap.
- Sources:
  - Tokio `select!` macro docs — https://docs.rs/tokio/latest/tokio/macro.select.html — Canonical explanation + cancellation safety notes.
  - Tokio tutorial: `select` — https://tokio.rs/tokio/tutorial/select — Practical patterns for multiplexing async streams.
  - `CancellationToken` (tokio-util) — https://docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken.html — Canonical cancellation primitive for coordinating shutdown.
  - Ratatui event handling concept — https://ratatui.rs/concepts/event-handling/ — Idiomatic event loop + routing guidance that maps to our reducer/effects model.

### Slack integration: APIs, real-time updates, and community-maintained Rust crates
- Best practices (synthesized):
  - Prefer Socket Mode + Events API for near real-time updates (and to avoid heavy polling), then use Web API calls for initial sync and occasional reconciliation.
  - Choose a Rust Slack library based on:
    - Coverage (Web API + Events + Socket Mode)
    - Typed models (Blocks/events/messages/files)
    - Community health (recent releases, docs, issue activity)
  - Treat rate limits as a design input:
    - Reduce calls to `conversations.history` / `conversations.replies` by relying on events to update local state incrementally, and by caching aggressively.
  - Options worth considering:
    - `slack-morphism` (recommended default): Web API + Events + Socket Mode + Block Kit coverage, typed models, and recent releases.
    - `slack-rs` (legacy): RTM-era approach; useful reference but not a modern foundation.
    - `slack-chat-api` (generated): can accelerate method coverage but freshness depends on the upstream spec source.
    - “Roll your own HTTP + models”: use `reqwest` + typed Block Kit model crates (`slack_morphism_models`, `slack_blocks`, `slack_messaging`) if we want maximum control.
  - Community health snapshot (as of this doc date, 2026-03-01):
    - slack-morphism shows a GitHub latest release in Feb 2026 and ongoing activity.
    - slack-rs shows a GitHub latest release in May 2020 (treat as legacy/reference only).
- Recommended default for this plan:
  - Use `slack-morphism` as the primary integration layer and implement Socket Mode early (Phase 2) as the real-time feed to keep the UI responsive while staying rate-limit aware.
- Pitfalls / footguns:
  - RTM is legacy and restricted for newer apps; an RTM-first strategy is likely to get blocked by platform constraints.
  - Tokens: Socket Mode uses an app-level token (`xapp-...`) for `apps.connections.open`, while Web API uses bot/user tokens; keep token types explicit in code and in docs.
- Sources:
  - slack-morphism (GitHub) — https://github.com/abdolence/slack-morphism-rust — Canonical repo + release activity and scope.
  - slack-morphism (docs.rs) — https://docs.rs/crate/slack-morphism/latest — API docs + examples entrypoint.
  - Slack Socket Mode docs — https://api.slack.com/apis/connections/socket — Official Socket Mode overview + requirements.
  - Slack Events API overview — https://api.slack.com/apis/connections/events-api — Slack’s recommended events delivery model (vs RTM).
  - Slack RTM restrictions (`rtm.start` changes) — https://api.slack.com/changelog/2021-10-rtm-start-to-stop — Explains RTM limitations and recommended alternatives.
  - Slack rate limit update + FAQ (2025-05) — https://api.slack.com/changelog/2025-05-terms-rate-limit-update-and-faq — Important for avoiding “poll history constantly” designs.
  - slack-rs (GitHub) — https://github.com/slack-rs/slack-rs — Demonstrates legacy RTM approach and maintenance window.

### Files + message rendering (mrkdwn / Block Kit) for a terminal client
- Best practices (synthesized):
  - Files:
    - Implement Slack’s modern 3-step upload flow (`files.getUploadURLExternal` → upload bytes to `upload_url` → `files.completeUploadExternal`).
    - Downloads of private files must include `Authorization: Bearer ...` when fetching `url_private(_download)`.
  - Message rendering:
    - Render from `blocks` first when present (Block Kit is the modern “source of truth” for layout), then fall back to `text` (mrkdwn).
    - Treat `attachments` as legacy/secondary.
    - Don’t assume CommonMark: Slack mrkdwn is Markdown-like but has Slack-specific tokens (`<@U...>`, `<#C...>`, `<http...|label>`). Implement these explicitly.
  - Rust helpers:
    - Use typed models for blocks/events to avoid emitting invalid payloads and to simplify parsing.
- Recommended default for this plan:
  - Implement the modern file upload flow from day one (do not build on deprecated `files.upload`).
  - Build two renderers:
    - A “Block Kit → terminal text” renderer for common blocks we care about.
    - A lightweight mrkdwn renderer that handles Slack tokens + minimal styling.
  - Use `slack_morphism_models` (or equivalent typed schemas) as the canonical data model source.
- Pitfalls / footguns:
  - `files.upload` is retired; relying on it will block new apps and break later.
  - Upload “uploaded vs shared”: an upload isn’t visible in a channel unless completed/shared correctly.
  - Sending Block Kit without `text` fallback is a common mistake; `text` is still important for notifications/accessibility.
- Sources:
  - Slack “Uploading files” guide — https://api.slack.com/messaging/files/uploading — Official 3-step upload flow and sharing semantics.
  - Slack changelog: better file uploads (and `files.upload` retirement context) — https://docs.slack.dev/changelog/2024-04-a-better-way-to-upload-files-is-here-to-stay — Official timeline and replacement APIs.
  - Slack `files.completeUploadExternal` — https://api.slack.com/methods/files.completeUploadExternal — Completion call semantics (including channel and thread behavior).
  - Slack `chat.postMessage` — https://api.slack.com/methods/chat.postMessage — Guidance on `text` vs `blocks` vs `attachments` when composing.
  - Slack formatting message text (mrkdwn) — https://docs.slack.dev/messaging/formatting-message-text/ — Slack-specific formatting and parsing rules.
  - Slack Block Kit overview — https://api.slack.com/block-kit — Canonical Block Kit reference.
  - slack_morphism_models (docs.rs) — https://docs.rs/slack-morphism-models — Typed Rust models for blocks/events/files/socket mode.

### Automation testing (headless + snapshot + PTY-driven E2E)
- Best practices (synthesized):
  - Treat “render” as pure: state → frame. Then you can test render output deterministically using Ratatui’s `TestBackend` (no terminal required).
  - Prefer headless/scripted driving of the **same action layer** as interactive mode (SSOT = `Action`): keys are just one frontend; headless commands are another.
  - For end-to-end “real terminal behavior” testing (escape sequences, raw mode edge cases), use a pseudo-terminal harness (PTY) and script it; don’t try to parse raw ANSI in ad-hoc ways.
  - Capture logs in tests explicitly (don’t rely on human-visible TUI output): use `tracing` and a test-friendly writer/layer.
  - For “live Slack” automation, keep it bounded and non-destructive:
    - connect Socket Mode + verify `hello` + ACK loop,
    - run a small number of Web API calls (e.g., `auth.test`, `conversations.list`),
    - optionally send a message only in a dedicated sandbox conversation (configured explicitly via env).
- Recommended default for this plan:
  - Primary automation path: build `--headless --script <path|->` that:
    - runs without a TTY,
    - executes a newline-delimited command script mapped to `Action`,
    - emits machine-readable JSON lines so tests can assert behavior without snapshots of raw terminal frames.
  - Secondary automation path (only if we need it): PTY-driven tests using `expectrl`/`portable-pty` plus a terminal parser (`vt100`) to assert high-level screen states.
- Pitfalls / footguns:
  - “Golden” screenshots of terminal output are brittle unless you normalize aggressively; prefer asserting structured state/log markers first.
  - PTY E2E tests can be flaky (timing); keep them minimal and add timeouts + deterministic scripts.
  - Live Slack E2E tests can mutate real workspaces; require explicit opt-in (env var) and a dedicated sandbox target.
- Sources:
  - Ratatui backends guide (TestBackend) — https://ratatui.rs/concepts/backends/ — Shows `TestBackend` as a first-class backend.
  - Ratatui recipe: testing apps — https://ratatui.rs/recipes/testing/ — Concrete examples for deterministic rendering tests.
  - expectrl (GitHub) — https://github.com/zhiburt/expectrl — PTY-style “expect” scripting for Rust programs.
  - portable-pty (docs.rs) — https://docs.rs/portable-pty/latest/portable_pty/ — Cross-platform PTY creation/IO (foundation for E2E TUI tests).
  - vt100 (docs.rs) — https://docs.rs/vt100/latest/vt100/ — Parse terminal output into a screen model for assertions.
  - tracing-test (docs.rs) — https://docs.rs/tracing-test/latest/tracing_test/ — Capture/assert `tracing` output in tests.
  - tracing-subscriber TestWriter — https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/test/struct.TestWriter.html — Test-friendly writer for captured logs.
  - Slack Socket Mode docs — https://api.slack.com/apis/connections/socket — Live connection + envelope ACK constraints.

## Adopt / Reject summary
- Adopt:
  - UI stack: `ratatui` + `crossterm`, with an explicit split-tree workspace and explicit focus routing.
  - Keybindings: `keybinds` for action dispatch + sequences; composer powered by `tui-textarea` (evaluate `edtui` only if we want deeper Vim editor behavior).
  - Async runtime: `tokio::select!`-driven non-blocking loop + cancellation tokens for clean shutdown (no hidden background tasks).
  - Slack integration: prefer `slack-morphism` and Socket Mode for near real-time updates.
  - Files: implement the modern 3-step upload flow; do authenticated downloads via `url_private_download`.
  - Automation: `--headless --script` driver mode emitting JSON lines; deterministic render tests via `TestBackend`; capture logs via `tracing` test helpers; add PTY E2E only if needed.
- Reject:
  - `tui-rs` as a dependency (archived); use Ratatui instead.
  - RTM-first approach for real-time (legacy/restricted; use Socket Mode).
  - `files.upload` for uploads (deprecated/retired; use the modern external upload flow).

## Open questions (ONLY if truly not answerable)
- (None — confirmed as a personal/internal tool. We should assume non-Marketplace constraints and keep Socket Mode + caching as the default posture.)
<!-- arch_skill:block:external_research:end -->

# 4) Current Architecture (as-is)

<!-- arch_skill:block:current_architecture:start -->
## 4.1 On-disk structure
```text
slack-rs/
  Cargo.toml
  Cargo.lock
  config.example.toml
  docs/
    TERMINAL_DRIVEN_RUST_SLACK_TUI_WITH_VIM_KEYBINDINGS_2026-03-01.md
    TERMINAL_DRIVEN_RUST_SLACK_TUI_WITH_VIM_KEYBINDINGS_2026-03-01_WORKLOG.md
  src/
    main.rs
    lib.rs
    app/
      action.rs
      app.rs
      config.rs
      headless.rs
      mod.rs
      reducer.rs
      runtime.rs
      state.rs
    diagnostics/
      banner.rs
      log.rs
      mod.rs
    input/
      keymap.rs
      mod.rs
      normalize.rs
      vim.rs
    render/
      mod.rs
    slack/
      mod.rs
      service.rs
      socket_mode.rs
      tokens.rs
    terminal/
      mod.rs
    ui/
      composer.rs
      mod.rs
      overlay.rs
      root.rs
      sidebar.rs
      theme.rs
      workspace.rs
    workspace/
      mod.rs
      ops.rs
      render.rs
      tree.rs
  tests/
    headless_smoke.rs
    headless_live_slack.rs
    workspace_tree.rs
```

## 4.2 Control paths (runtime)
- Flow A — Interactive startup + UI loop:
  - `src/main.rs` loads `.env` (best-effort) and dispatches to interactive mode by default.
  - `src/app/app.rs`:
    - loads TOML config (`src/app/config.rs`) and initializes the key dispatcher (`src/input/keymap.rs`),
    - enters terminal raw mode + alternate screen (`src/terminal/mod.rs`),
    - starts a background tokio runtime (`src/app/runtime.rs`) and spawns Socket Mode (`src/slack/socket_mode.rs`),
    - runs the main render/input loop: draw UI (`src/ui/root.rs`) → read keys (`crossterm`) → map to `Action` → apply reducer (`src/app/reducer.rs`).
- Flow B — Headless non-interactive driver (automation-first):
  - `src/main.rs` with `--headless --script <path|->` runs `src/app/headless.rs`.
  - `src/app/headless.rs`:
    - marks headless mode (`src/terminal/mod.rs`) so raw mode/alt-screen becomes an explicit error if called (trap check),
    - reads newline-delimited script commands,
    - executes:
      - pure UI actions (mapped to `src/app/action.rs`), and
      - opt-in live Slack smoke commands (`slack_auth_test`, `slack_socket_mode_smoke <timeout_ms>`),
    - emits JSON lines to stdout per step (`started` / `state` / `slack_*` / `quit`) so agents/CI can assert results without a TTY.
- Flow C — Socket Mode background task:
  - `src/slack/service.rs` calls `apps.connections.open` to obtain the websocket URL.
  - `src/slack/socket_mode.rs` connects via `tokio_tungstenite`, ACKs `envelope_id` immediately, and forwards lightweight `AppEvent`s to the UI loop.
  - Fail-fast: websocket close/disconnect/errors are fatal (no reconnect).

## 4.3 Object model + key abstractions
- `src/app/state.rs::AppState` — SSOT for UI state (mode, workspace, composer) and Socket Mode connection status.
- `src/workspace/tree.rs::PaneTree` — SSOT split-tree for the right-side workspace (nested splits + focused leaf).
- `src/app/action.rs::Action` — small “drive surface” for core UI operations.
- `src/app/reducer.rs::apply_action` — pure state transitions (no Slack I/O; no terminal I/O).
- `src/input/keymap.rs::KeyDispatcher` — mode-aware mapping from key sequences to `Action` (TOML-configurable).
- `src/slack/tokens.rs::SlackTokens` — `APP_TOKEN` + `BOT_TOKEN` from env, prefix validated, debug redacted.
- `src/slack/service.rs::SlackService` — Slack Web API boundary (currently: `socket_mode_url`, `auth_test_bot`, `list_sidebar_conversations`, `fetch_conversation_history`).
- `src/slack/socket_mode.rs` — Socket Mode receive loop + “hello” smoke-connect for automation.

## 4.4 Observability + failure behavior today
- Logs: `src/diagnostics/log.rs` initializes `tracing_subscriber` with env filtering.
- Fail-fast behavior:
  - `src/main.rs` restores the terminal (best-effort) and prints a clear fatal error banner on any error path.
  - Socket Mode is explicitly fail-fast (no reconnect/retry loops).
  - Headless mode is explicitly fail-fast and emits machine-readable progress markers up to the point of failure.
- Secrets safety:
  - Tokens are loaded from env (`APP_TOKEN`, `BOT_TOKEN`) and are never printed.
  - `SlackTokens` debug output is redacted (`"<redacted>"`).

## 4.5 UI surfaces (ASCII mockups, if UI work)

```ascii
┌slack-rs  |  SocketMode: connected (N events)  |  [NORMAL]──────────────────────┐
│┌Slack bar────────────────┐┌Pane 1 (focused)─────────────────────────────────┐│
││Threads (recent/open)    ││PaneKind::Empty (placeholder)                     ││
││  (none yet)             ││                                                 ││
││                         ││(workspace supports split/focus/close via keys)   ││
││Channels                 ││                                                 ││
││  * #eng                 ││                                                 ││
││    #random              ││                                                 ││
││                         ││                                                 ││
││DMs                      ││                                                 ││
││    @sara                ││                                                 ││
││    @dave                ││                                                 ││
│└─────────────────────────┘└─────────────────────────────────────────────────┘│
│┌Compose (press i to edit)────────────────────────────────────────────────────┐│
││<textarea>                                                                   ││
│└─────────────────────────────────────────────────────────────────────────────┘│
└──────────────────────────────────────────────────────────────────────────────┘
```
<!-- arch_skill:block:current_architecture:end -->

---

# 5) Target Architecture (to-be)

<!-- arch_skill:block:target_architecture:start -->
## 5.1 On-disk structure (future)

```text
slack-rs/
  Cargo.toml
  Cargo.lock
  config.example.toml
  docs/
    TERMINAL_DRIVEN_RUST_SLACK_TUI_WITH_VIM_KEYBINDINGS_2026-03-01.md
    TERMINAL_DRIVEN_RUST_SLACK_TUI_WITH_VIM_KEYBINDINGS_2026-03-01_WORKLOG.md
  src/
    main.rs
    lib.rs
    app/
      action.rs                # SSOT: user-visible actions (keys + headless scripts drive this)
      reducer.rs               # pure state transitions (no I/O)
      state.rs                 # SSOT app state (sidebar + panes + caches + composer)
      app.rs                   # interactive UI loop (crossterm + ratatui)
      headless.rs              # non-interactive driver (newline script → JSON lines)
      runtime.rs               # background tokio runtime + event channel for Slack I/O
      config.rs                # TOML config load + keybind validation
      effects.rs               # SSOT side-effects boundary (Slack I/O + filesystem I/O)
      mod.rs
    diagnostics/
      banner.rs
      log.rs
      mod.rs
    input/
      keymap.rs
      vim.rs
      normalize.rs
      mod.rs
    model/
      mod.rs                   # internal Slack model (stable, UI-friendly types; avoid vendor types)
      conversation.rs
      message.rs
      thread.rs                # (planned) thread model
      user.rs
    render/
      mod.rs
      blocks.rs                # (planned) Block Kit renderer
      mrkdwn.rs                # (planned) Slack mrkdwn renderer
    slack/
      mod.rs
      tokens.rs
      service.rs
      socket_mode.rs
      events.rs                # (planned) Socket Mode payload → internal events/actions
      files.rs                 # (planned) modern upload/download helpers
    terminal/
      mod.rs
    ui/
      root.rs
      sidebar.rs
      workspace.rs
      composer.rs
      overlay.rs
      theme.rs
      mod.rs
    workspace/
      tree.rs
      ops.rs
      render.rs
      mod.rs
  tests/
    headless_smoke.rs
    headless_live_slack.rs     # ignored; requires SLACK_LIVE_TEST=1 + real tokens
    workspace_tree.rs
    ui_snapshot_smoke.rs       # (planned) TestBackend render snapshots (no TTY)
    e2e_live_slack_workflows.rs # (planned) ignored; safe, bounded live Slack workflows
```

## 5.2 Control paths (future)

* Flow A — Interactive startup + render loop (no network on the render path):
  * `src/main.rs` → `src/app/app.rs::App::run()`
  * Load config (`src/app/config.rs`) → initialize key dispatcher (`src/input/keymap.rs`)
  * Enter terminal raw mode + alternate screen (`src/terminal/mod.rs::TerminalGuard::enter`)
  * Spawn background Slack tasks (`src/app/runtime.rs`, `src/slack/socket_mode.rs`) and forward results into the UI loop as explicit events/actions.
  * Loop: drain runtime events → draw UI (`src/ui/root.rs`) → read key events → map to `Action` → apply reducer (`src/app/reducer.rs`).
* Flow B — Headless non-interactive driver (automation SSOT):
  * `src/main.rs --headless --script <path|->` → `src/app/headless.rs::run_headless`
  * Script lines drive `Action` transitions (pane ops, focus, compose, quit) and explicit Slack smoke commands.
  * Emit JSON lines to stdout so tests/agents can assert behavior without a TTY.
* Flow C — Live Slack E2E automation (ignored by default; still non-interactive):
  * `tests/headless_live_slack.rs` runs the headless binary and asserts:
    * Web API connectivity (`slack_auth_test`)
    * Socket Mode connectivity (`slack_socket_mode_smoke`)
  * As we add features, expand ignored live tests for safe workflows (send, reply, react, upload/download) gated by explicit env vars and cleanup rules.
* Flow D — Deterministic UI snapshot tests (no Slack; no TTY):
  * Render `AppState` fixtures via Ratatui `TestBackend` and assert high-level layout/visibility (sidebar sections, pane borders/focus, composer state).
  * This enables “per-phase automation first” without fragile PTY screenshot scraping.

## 5.3 Object model + abstractions (future)

* Key types/modules (current + planned growth):
  * `src/app/state.rs::AppState` — SSOT for UI + automation-observable state:
    * workspace split-tree + focus (`src/workspace/*`)
    * composer buffer + mode (`src/ui/composer.rs`)
    * Socket Mode connectivity status (healthy vs fatal)
    * (planned) sidebar lists + message caches (channels/DMs/threads, timeline pages)
  * `src/workspace/tree.rs::PaneTree` — SSOT for nested splits + focused leaf.
  * `src/app/action.rs::Action` — the stable “drive surface” for user-visible behavior; both:
    * interactive keybindings (`src/input/keymap.rs`), and
    * non-interactive scripts (`src/app/headless.rs`)
    map into this.
  * `src/app/reducer.rs::apply_action` — pure state transitions (no Slack I/O, no terminal I/O, no filesystem I/O).
  * `src/app/headless.rs` — headless driver:
    * parses newline scripts into commands,
    * runs `Action` transitions via the reducer,
    * runs explicit live Slack smoke commands when requested,
    * emits JSON lines for assertions.
  * `src/slack/tokens.rs::SlackTokens` — env-only tokens (`APP_TOKEN`, `BOT_TOKEN`), prefix validated, debug redacted.
  * `src/slack/service.rs::SlackService` — Slack Web API boundary (current: `socket_mode_url`, `auth_test_bot`; planned: conversations/history/post/reactions/files).
  * `src/slack/socket_mode.rs` — Socket Mode loop (`run_socket_mode`) + automation connectivity check (`smoke_connect`).
  * `src/model/*` (planned) — internal Slack model types for caches + stable test fixtures (avoid coupling tests to slack-morphism structs).
  * `src/render/*` (planned) — pure message rendering (mrkdwn/blocks → terminal spans/lines).
* Explicit contracts (automation-first):
  * Reducer purity:
    * `src/app/reducer.rs` remains pure and unit-testable; side effects must be modeled explicitly (planned: `src/app/effects.rs`).
  * Non-interactive E2E as a build constraint:
    * Any new interactive feature must be scriptable via `--headless --script` (either as an `Action` sequence or a single command).
    * Each implementation phase must ship at least one automation signal (unit/integration/headless script/live Slack ignored E2E).
    * Manual testing exists only as a finalization checklist; it must not be a mid-phase gate.
  * Live Slack automation safety:
    * Live tests are ignored by default and require explicit opt-in (`SLACK_LIVE_TEST=1`) + real tokens.
    * Default posture is read-only when possible; bounded writes require explicit cleanup rules and should target a dedicated channel/thread.
  * Deterministic machine-readable output:
    * Headless mode emits JSON lines with stable keys so automation doesn’t scrape terminal output.
* Public APIs (current + planned):
  * Interactive:
    * `src/app/app.rs::App::run() -> anyhow::Result<()>`
  * Automation:
    * `src/app/headless.rs::run_headless(HeadlessArgs) -> anyhow::Result<()>`
    * `src/app/reducer.rs::apply_action(state, action) -> anyhow::Result<ReduceOutcome>`
  * Slack boundary (current):
    * `src/slack/service.rs::SlackService::socket_mode_url() -> anyhow::Result<String>`
    * `src/slack/service.rs::SlackService::auth_test_bot() -> anyhow::Result<AuthTestSummary>`
    * `src/slack/service.rs::SlackService::list_sidebar_conversations() -> anyhow::Result<SidebarConversations>`
    * `src/slack/service.rs::SlackService::fetch_conversation_history(...) -> anyhow::Result<HistoryPage>`
    * `src/slack/socket_mode.rs::run_socket_mode(...) -> anyhow::Result<()>`
    * `src/slack/socket_mode.rs::smoke_connect(...) -> anyhow::Result<()>`
  * Slack boundary (planned expansion):
    * thread replies; post/reply; add/remove reactions; upload/download files.

## 5.4 Invariants and boundaries

* Automation-first verification (per-phase; no humans mid-flight):
  * Every phase must ship at least one non-interactive automation signal:
    * pure unit tests (workspace/keymap/reducer),
    * headless `--script` smoke checks (stdout JSON assertions),
    * and (when relevant) ignored live Slack E2E checks gated by explicit env vars.
  * Manual testing exists only as a finalization checklist at the end; it must not be the primary verification mechanism while building.
  * Live Slack E2E tests must be opt-in, bounded, and safe-by-default (read-only when possible; dedicated channels/threads for any writes + explicit cleanup).
* Fail-fast boundaries:
  * Slack errors must be explicit and actionable (error chain + context), and then the app should exit/crash after restoring the terminal — no “keep running” recovery that masks problems.
  * Unsupported message content (unknown Block Kit blocks, attachments) must render as explicit placeholders (“unsupported block type: …”), not disappear.
  * Keybinding ambiguity is solved via configurability first (don’t assume modified keys are distinguishable); only add a diagnostics UI if terminal quirks block progress.
* Single source of truth:
  * `AppState` is the SSOT; UI reads from it, reducer updates it, effects never mutate it directly.
  * `PaneTree` is the SSOT for splits/focus; UI layout is derived from it every frame.
* Determinism contracts (time/randomness):
  * Key sequence dispatch must be deterministic: timeouts are handled in one place (input layer) with monotonic time (`Instant`).
  * Tests for keymaps/workspace ops should be pure and not depend on wall clock or network.
  * Headless automation output is machine-readable JSON with stable keys; automation must not scrape full-screen terminal output.
* Performance / allocation boundaries:
  * No network I/O on the render path.
  * Message lists should be virtualized (render only visible lines) and cache rendered text where it’s expensive (Block Kit + mrkdwn).
  * Avoid unbounded caches: cap the in-memory window per conversation, but allow reaching full history via “load older” pagination when scrolling up.
  * Background tasks must not flood the action channel; apply backpressure (bounded channels) and coalesce where it makes sense (e.g., bursty Slack events).

## 5.5 UI surfaces (ASCII mockups, if UI work)

```ascii
Legend:
  - Left sidebar is the “Slack bar”: Channels + DMs + Threads (recent/open; always visible).
  - Right side is the “workspace canvas”: arbitrarily split panes (nested splits).
  - Borders with **DOUBLE-LINE** imply the focused pane (only one focused pane at a time).
  - Composer is global and sends to the focused pane’s context (channel/DM/thread).
  - Leader key default: `Space` (Vim-like leader sequences; configurable).
  - Composer behavior: `Enter` sends; `Shift+Enter` inserts newline (terminal support varies; bindings are configurable).
  - No “which-key” help UI in v0.1.0; the UI should stay focused on navigation + reading + composing.
  - Defaults shown below are configurable (Vim-style).

--------------------------------------------------------------------------------
Screen A — Default: single focused pane (Channel timeline) + global composer

+--------------------------+------------------------------------------------------+
| slack-rs  (ACME)         | #eng  (SocketMode: OK)                 [NORMAL]       |
|--------------------------+------------------------------------------------------|
|                          | 10:14 alice  Shipping v0.1 plan is ready.             |
|--------------------------|             - split panes + vim keys first            |
| Threads (recent/open) (2)|             - Socket Mode early (avoid polling)       |
|  * #eng / alice: ...     |                                                     |
|    #infra / bob: ...     | 10:16 bob    Here's the failure log:                  |
|--------------------------|   ```                                                 |
| Channels                  |   error: ...                                          |
|  * #eng           (12)   |   ```                                                 |
|    #random         (3)   |   :+1: 3   :eyes: 1   :partyparrot: 2                 |
|    #infra                |                                                     |
|--------------------------| 10:19 sara   file: build.log (12 KB)  [d] download    |
| DMs                      |            snippet: stacktrace.txt      [o] open       |
|    @dave           (1)   |                                                     |
|    @sara                | 10:21 you    ok — will fix.                           |
|--------------------------|------------------------------------------------------|
| Compose → #eng (focused pane)                                                |
| >                                                                            |
+--------------------------+------------------------------------------------------+

--------------------------------------------------------------------------------
Screen B — Vertical split: Channel timeline + Thread view side-by-side

+--------------------------+-------------------------------+------------------------+
| slack-rs  (ACME)         | #eng (Channel)                || Thread (focused)     ||
|--------------------------+-------------------------------|| (reply context)      ||
|                          | 10:14 alice  Shipping v0.1... || Parent:               ||
|--------------------------| 10:16 bob    Here's the log.. || 10:14 alice ...       ||
| Threads (recent/open) (2)| 10:19 sara   file: build.log  ||-----------------------||
|  * #eng / alice: ...     | 10:21 you    ok — will fix    || 10:15 you  +1         ||
|--------------------------|                               || 10:18 bob  ack        ||
| Channels                 |                               || 10:22 you  reply...  ||
|  * #eng           (12)   |                               ||                       ||
| DMs                      |                               ||                       ||
|    @sara                |                               ||                       ||
+--------------------------+-------------------------------+------------------------+
| Compose → Thread in #eng (focused pane)                                        |
| >                                                                             |
+------------------------------------------------------------------------------+

--------------------------------------------------------------------------------
Screen C — Nested splits: Channel + Thread + DM (three panes, arbitrary layout)

+--------------------------+--------------------------------------+----------------------+
| slack-rs  (ACME)         | #eng (Channel)                       | @sara (DM)          |
|--------------------------+--------------------------------------+----------------------|
|                          | 10:14 alice  Shipping v0.1 plan...   | 09:58 sara  morning |
|--------------------------+ 10:16 bob    Here's the log...        | 10:02 you   yep     |
| Threads (recent/open) (2)| 10:19 sara   file: build.log          | 10:07 sara  thread? |
|  * #eng / alice: ...     |--------------------------------------|----------------------|
|    #infra / bob: ...     || Thread (focused)                     |                      |
|--------------------------|| Parent: 10:14 alice ...              |                      |
| Channels                 || 10:15 you   reply...                  |                      |
|  * #eng           (12)   || 10:18 bob   ack                       |                      |
|    #infra                || 10:22 you   reply...                  |                      |
| DMs                      |--------------------------------------|----------------------|
|  * @sara                 |                                      |                      |
|    @dave           (1)   |                                      |                      |
+--------------------------+--------------------------------------+----------------------+
| Compose → Thread in #eng (focused pane)                                        |
| >                                                                             |
+------------------------------------------------------------------------------+

--------------------------------------------------------------------------------
Screen D — Fatal error (fail-fast; no recovery; terminal restored on exit)

+--------------------------+------------------------------------------------------+
| slack-rs  (ACME)         | FATAL ERROR                                            |
|--------------------------+------------------------------------------------------|
| Sidebar ...              | Something went wrong and slack-rs is exiting.          |
|                          |                                                       |
|                          |  Error chain:                                          |
|                          |   1) Socket Mode disconnected: EOF                     |
|                          |   2) while reading websocket frame                     |
|                          |                                                       |
|                          |  Next steps:                                           |
|                          |   - check tokens/scopes in config                      |
|                          |   - rerun with `RUST_BACKTRACE=1`                      |
|                          |   - if input is weird, remap keys in config            |
|                          |                                                       |
|                          |                                                       |
+--------------------------+------------------------------------------------------+
| Press any key to exit                                                         |
+------------------------------------------------------------------------------+
```
<!-- arch_skill:block:target_architecture:end -->

---

# 6) Call-Site Audit (exhaustive change inventory)

<!-- arch_skill:block:call_site_audit:start -->
## 6.1 Change map (table)

| Area | File | Symbol / Call site | Current behavior | Required change | Why | New API / contract | Tests impacted |
| ---- | ---- | ------------------ | ---------------- | --------------- | --- | ------------------ | -------------- |
| CLI entrypoint | `src/main.rs` | `parse_mode_and_run()` | Interactive by default; `--headless --script <path|->` runs non-interactively | Keep headless mode stable; extend flags only when a new automation surface is required | automation SSOT | “headless is always available” invariant | `tests/headless_smoke.rs`, `tests/headless_live_slack.rs` |
| Headless driver | `src/app/headless.rs` | `run_headless(...)` | Parses script → runs `Action` reducer + live Slack smoke commands; emits JSON lines | Expand command surface as features land (open convo, paginate, send/reply, react, files) while keeping JSON output stable | end-to-end automation | stable script protocol + JSON schema | headless tests + future ignored E2E workflows |
| Headless safety trap | `src/terminal/mod.rs` | `mark_headless_mode()`, `TerminalGuard::enter()` | Headless marks mode; any attempt to enter raw mode becomes an explicit error | Keep as invariant; prevents accidental “headless but actually interactive” behavior | determinism | “no TTY required in headless” | covered by headless smoke; add regression test only if it breaks |
| Action SSOT | `src/app/action.rs` | `enum Action` | Core UI actions (pane ops, compose, quit) | Add Slack-facing actions (open conversation/thread, paginate, react, file ops) without baking behavior into UI code | testability + reuse | “everything is an Action + effects” | unit tests for reducer + headless scripts |
| Reducer SSOT | `src/app/reducer.rs` | `apply_action(...)` | Pure state transitions; compose send/newline semantics and pane ops | Keep pure; add new transitions for sidebar selection + timeline state | automation friendliness | reducer purity | unit tests; headless scripts validate state markers |
| Interactive UI loop | `src/app/app.rs` | `App::run()` | crossterm input + ratatui draw loop; Slack runs in background; fail-fast on fatal events | Keep UI draw pure (state → frame) and keep Slack I/O off render path as features expand | responsiveness | “no Slack I/O on render path” | snapshot tests (planned) + headless E2E (preferred) |
| Sidebar rendering | `src/ui/sidebar.rs` | `draw_sidebar(...)` | Placeholder static Channels/DMs/Threads view | Drive from `AppState` sidebar model fed by Slack API; keep rendering pure | core UX | sidebar derives from state only | snapshot tests (planned) |
| Workspace SSOT | `src/workspace/*` | `PaneTree`, `ops::*`, `render::layout` | Nested splits + focus + close; pure + tested | Extend panes to represent channel/DM/thread timeline types + per-pane view state | core differentiator | “split tree is SSOT” | `tests/workspace_tree.rs` + headless pane ops |
| Slack tokens | `src/slack/tokens.rs` | `SlackTokens::from_env()` | Validates `APP_TOKEN`/`BOT_TOKEN` prefixes; redacted debug | Keep env-only; never move into TOML; add scopes notes in docs as needed | secret safety | token contract | unit tests (optional) |
| Slack Web API boundary | `src/slack/service.rs` | `SlackService::{socket_mode_url, auth_test_bot}` | Minimal Slack Web API wrapper | Add conversations/history/post/replies/reactions/files behind this boundary | Slack parity | one Slack boundary | ignored live Slack E2E tests (per feature) |
| Socket Mode realtime | `src/slack/socket_mode.rs` | `run_socket_mode(...)`, `smoke_connect(...)` | ACK fast; track connected/event count; fail-fast on disconnect; smoke_connect waits for `hello` | Add event translation → internal actions + caches; keep smoke_connect as minimal live check | realtime + automation | “ACK fast; no reconnect” | `tests/headless_live_slack.rs` + future live event tests |
| Automation tests | `tests/*` | `headless_smoke`, `headless_live_slack`, `workspace_tree` | Non-interactive smoke exists; live Slack smoke exists (ignored); pure workspace tests exist | Add per-phase automation tests first; keep manual tests only at finalization | speed | automation gating | expand suite as phases land |

## 6.2 Migration notes

* Deprecated APIs (if any): N/A (new system).
* Delete list:
  * Keep the automation surface single-sourced: do not add a second “driver” path that bypasses `Action`/reducer.

## 6.3 Pattern Consolidation Sweep (anti-blinders; scoped by plan)

| Area | File / Symbol | Pattern to adopt | Why (drift prevented) | Proposed scope (include/defer/exclude) |
| ---- | ------------- | ---------------- | ---------------------- | ------------------------------------- |
| automation SSOT | `src/app/action.rs`, `src/app/reducer.rs`, `src/app/headless.rs` | “keys are a frontend; scripts are a frontend; Action is SSOT” | prevents drift between interactive and automated behavior | include |
| headless safety | `src/terminal/mod.rs` | headless trap check | prevents accidental raw-mode entry in automation | include |
| pane layout | `src/workspace/tree.rs` | split-tree SSOT | prevents ad-hoc split logic in UI code | include |
| pane ops | `src/workspace/ops.rs` | pure ops + tests | keeps focus/splits deterministic | include |
| input routing | `src/input/vim.rs` | precedence rules (overlay → focused input → keymap) | prevents “keys leak into wrong widget” bugs | include |
| Slack I/O boundary | `src/slack/*` (+ planned `src/app/effects.rs`) | all Slack I/O behind SlackService/effects | prevents hidden network calls in UI/draw code | include |
| errors | `src/diagnostics/banner.rs` | fail-fast surfacing | prevents masked bugs | include |
| live Slack E2E hygiene | `tests/headless_live_slack.rs` (+ future ignored tests) | explicit env gates + safe workflows | prevents accidental workspace mutation and flaky CI | include |
<!-- arch_skill:block:call_site_audit:end -->

---

# 7) Depth-First Phased Implementation Plan (authoritative)

<!-- arch_skill:block:phase_plan:start -->
<!--
arch_skill:phase_plan_granularize
level: 3
pass_count: 4
last_updated: 2026-03-02
-->

> Rule: systematic build, foundational first; every phase has exit criteria + explicit verification plan (tests optional). No fallbacks/runtime shims — the system must work correctly or fail fast with an explicit error chain (delete legacy/parallel paths, don’t shim them). Prefer programmatic checks per phase and require at least one non-interactive automation signal (unit/integration/headless/live Slack ignored E2E) per phase; keep manual/UI verification only as a finalization checklist. Avoid negative-value tests (deletion checks, visual constants, doc-driven gates). Document new patterns/gotchas at the canonical boundary modules (high leverage; no comment spam).

## Phase 0 — Preflight: crate skeleton + baseline checks

* Goal:
  * Establish a clean Rust crate + baseline commands so every later change is additive and easy to verify (`cargo build`, `cargo test`, `cargo run`).
* Work (microtasks):
  - [x] P0.T1 — Create crate + baseline dependency set
    - File anchors: `Cargo.toml`, `src/main.rs`
    - Steps:
      - Create a binary crate (single package is fine at first).
      - Add baseline deps: `ratatui`, `crossterm`, `tokio`, `anyhow`, `tracing`, `tracing-subscriber`, `serde`, `toml`, `directories`.
      - Add a minimal `main()` that exits cleanly (no raw mode yet).
    - Verification (smallest signal): `cargo build` and `cargo test`
    - Exit criteria: `cargo test` passes in a clean checkout.
    - Rollback: `git reset --hard` (no external state).
  - [x] P0.T2 — Lay down the “future tree” as empty modules (compile-only)
    - File anchors: `src/app/mod.rs`, `src/input/mod.rs`, `src/workspace/mod.rs`, `src/ui/mod.rs`, `src/slack/mod.rs`, `src/render/mod.rs`, `src/diagnostics/mod.rs`
    - Steps:
      - Create empty modules + `mod.rs` files to match the target architecture.
      - Keep everything stubbed but compiling (no functionality required).
    - Verification (smallest signal): `cargo check`
    - Exit criteria: target module tree exists without fighting the compiler.
    - Rollback: revert commit.
* Verification (smallest signal):
  * `cargo check` is fast and reliable — we’ll use it constantly.
* Exit criteria:
  * New agent can run `cargo check` and get a deterministic result.
* Rollback:
  * Revert commits.

## Phase 1 — Repo bootstrap + runnable TUI skeleton (no Slack yet)

* Goal:
  * Have a `cargo run`-able full-screen TUI that renders the basic layout (sidebar + workspace canvas + status + composer area) and always restores the terminal correctly on exit/crash.
* Work (microtasks):
  - [x] P1.T1 — Implement the “terminal boundary” (raw mode + alt screen + always-restore)
    - File anchors: `src/terminal/mod.rs` (new), `src/main.rs`
    - Steps:
      - Create a single owner type (e.g., `TerminalGuard`) that enters raw mode + alternate screen.
      - Ensure restore happens on normal exit and on panic (panic hook or Drop guard + best-effort restore).
    - Verification (smallest signal): `cargo run` → quit → terminal echo/raw mode is normal
    - Exit criteria: you can `cargo run` / quit repeatedly without terminal corruption.
    - Rollback: revert commit.
  - [x] P1.T2 — Add app runtime skeleton (`App::run`) with a deterministic quit path
    - File anchors: `src/app/app.rs` (new), `src/app/mod.rs`, `src/main.rs`
    - Steps:
      - Define `App` + `App::run()` as the only entry to the event loop.
      - Handle quit via `q` and `Ctrl+c` (hard-coded at first is fine; keymap comes in Phase 2).
    - Verification (smallest signal): `cargo run` → `q` exits cleanly
    - Exit criteria: raw-mode UI always exits on quit keys.
    - Rollback: revert commit.
  - [x] P1.T3 — Render the root layout regions (placeholders only)
    - File anchors: `src/ui/root.rs` (new), `src/ui/mod.rs`
    - Steps:
      - Draw the left sidebar region (“Slack bar”) and right workspace canvas region.
      - Draw a status/banner region and a composer region (even if static).
    - Verification (smallest signal): `cargo run` shows the correct region geometry
    - Exit criteria: layout matches the wireframe boundaries (no overlapping widgets).
    - Rollback: revert commit.
  - [x] P1.T4 — Add fail-fast fatal error output (error chain + terminal restore)
    - File anchors: `src/diagnostics/banner.rs` (new), `src/app/state.rs` (new)
    - Steps:
      - Add a single “fatal error” path that:
        - restores the terminal (even on panic),
        - prints an explicit error chain (and backtrace when available),
        - then exits/crashes (no recovery).
      - Keep the mechanism simple and centralized (no scattered error printing).
    - Verification (smallest signal): intentionally fail startup → fatal error is shown; terminal is restored after exit
    - Exit criteria: when it breaks, it breaks loudly and leaves the terminal clean.
    - Rollback: revert commit.
  - [x] P1.T5 — Delete/cleanup: forbid ad-hoc terminal mode toggling
    - File anchors: `src/terminal/mod.rs`, `src/main.rs`, `src/app/app.rs`
    - Steps:
      - Ensure only the terminal boundary module touches raw mode / alternate screen.
      - Keep the terminal API small and hard to misuse.
    - Verification (smallest signal): `rg \"enable_raw_mode|disable_raw_mode|EnterAlternateScreen|LeaveAlternateScreen\" -n src` shows only the terminal module
    - Exit criteria: terminal invariants are hard to violate accidentally.
    - Rollback: n/a (structural).
* Verification (smallest signal):
  * `cargo run` opens the UI; quitting returns your terminal to normal.
* Exit criteria:
  * You can repeatedly run/quit without terminal corruption.
  * The screen layout matches the high-level wireframe regions.
* Rollback:
  * Revert commits.

## Phase 2 — Split-pane workspace tree + focus routing + keymap/modes + composer widget

* Goal:
  * Make the core differentiator real: arbitrary nested splits on the right, deterministic focus movement, and configurable Vim-style keybindings driving it.
* Work (microtasks):
  - [x] P2.T0 — Add input/keybinding dependencies (`keybinds`, `tui-textarea`)
    - File anchors: `Cargo.toml`
    - Steps:
      - Add `keybinds` for key sequences/timeouts and `tui-textarea` for the composer.
      - Keep feature flags minimal; we can expand only when forced.
    - Verification (smallest signal): `cargo check`
    - Exit criteria: dependencies compile on your machine.
    - Rollback: revert commit.
  - [x] P2.T1 — Implement the split-tree SSOT (`PaneTree`) + stable IDs
    - File anchors: `src/workspace/tree.rs` (new)
    - Steps:
      - Represent splits (H/V) and leaves with stable identifiers.
      - Encode “focus always points to a leaf” as a first-class invariant.
    - Verification (smallest signal): `cargo test -q` (workspace tests)
    - Exit criteria: tree can represent nested splits without UI code involvement.
    - Rollback: revert commit.
  - [x] P2.T2 — Implement pure workspace ops (split/focus/close/resize)
    - File anchors: `src/workspace/ops.rs` (new), `src/workspace/tree.rs`
    - Steps:
      - Implement: split vertical/horizontal, focus movement, close focused, resize focused.
      - Keep ops pure (no ratatui types, no I/O).
    - Verification (smallest signal): `cargo test -q`
    - Exit criteria: ops are deterministic and preserve invariants.
    - Rollback: revert commit.
  - [x] P2.T3 — Lock workspace invariants with fast tests
    - File anchors: `tests/workspace_tree.rs` (new)
    - Steps:
      - Add tests for: focus leaf invariant, split preserves existing leaf as one side, close produces valid tree.
    - Verification (smallest signal): `cargo test -q`
    - Exit criteria: split-tree regressions are caught without running the UI.
    - Rollback: revert commit.
  - [x] P2.T4 — Render `PaneTree` into ratatui layout constraints (UI derives layout)
    - File anchors: `src/workspace/render.rs` (new), `src/ui/root.rs`
    - Steps:
      - Convert tree → rectangles for leaves.
      - Render each leaf as a labeled placeholder (“pane #3: channel”, etc.).
    - Verification (smallest signal): `cargo run` + manual split keys (wired temporarily) shows nested panes
    - Exit criteria: nested splits render without geometry glitches.
    - Rollback: revert commit.
  - [x] P2.T5 — Normalize terminal input events into an internal key representation
    - File anchors: `src/input/normalize.rs` (new)
    - Steps:
      - Convert `crossterm::event::Event` into a stable internal `KeyEvent` / `KeyChord`.
      - Preserve modifier info (Ctrl/Alt/Shift) and (when debug logging is enabled) emit normalized events to logs for quick troubleshooting.
    - Verification (smallest signal): `cargo run` + debug logging shows normalized key events for a few keystrokes
    - Exit criteria: key normalization is deterministic across the app.
    - Rollback: revert commit.
  - [x] P2.T6 — Keymap config schema + load/validate at startup
    - File anchors: `src/app/config.rs` (new), `config.example.toml` (new)
    - Steps:
      - Define a minimal set of commands: pane splits, focus movement, close, quit.
      - Parse `config.toml` from an XDG-appropriate location (`directories` crate), with a local-first `config.toml` override for development.
    - Verification (smallest signal): invalid config → fail loudly with actionable message
    - Exit criteria: bindings are configurable without code edits.
    - Rollback: delete local config file; revert commit.
  - [x] P2.T7 — Vim-ish keybinding engine + deterministic precedence rules
    - File anchors: `src/input/keymap.rs`, `src/app/app.rs`, `src/app/state.rs`, `src/ui/composer.rs`
    - Steps:
      - Support multi-key sequences (e.g., `Ctrl+w` then `v`).
      - Implement precedence so keys never leak into the wrong widget: (future modal prompts) → focused input widget (composer) → mode keymap → no-op.
    - Verification (smallest signal): manual: `Ctrl+w v/s`, `Ctrl+w h/j/k/l`, `Ctrl+w q` all work; composer keys don’t trigger pane ops
    - Exit criteria: “what does this key do?” is predictable.
    - Rollback: revert commit.
  - [x] P2.T8 — Composer widget (multi-line) with send semantics (no help UI yet)
    - File anchors: `src/ui/composer.rs`, `src/app/state.rs`, `config.example.toml`
    - Steps:
      - Use `tui-textarea` for multi-line input.
      - Implement: `Enter` sends, `Shift+Enter` inserts newline (configurable via `keybinds.composer_send` / `keybinds.composer_newline`).
      - Fail fast if `Enter` send could be ambiguous with `Shift+Enter` newline (require observing a real `Shift+Enter` event once before allowing `Enter` send; otherwise crash with an actionable message).
    - Verification (smallest signal): manual: insert newline; “send” triggers debug banner (no Slack yet)
    - Exit criteria: composer is usable and doesn’t fight pane navigation.
    - Rollback: revert commit.
  - [x] P2.T9 — Delete/cleanup: remove any hard-coded key handling outside the input layer
    - File anchors: `src/app/app.rs`, `src/input/*`
    - Steps:
      - Ensure all key → command routing flows through the keymap engine (except emergency quit).
    - Verification (smallest signal): update `config.toml` binding → behavior changes
    - Exit criteria: keymap is SSOT for behavior.
    - Rollback: revert commit.
  - [x] P2.T10 — Wire pane resize into keymap + reducer (automation-visible)
    - File anchors: `src/app/action.rs`, `src/app/config.rs`, `src/input/keymap.rs`, `config.example.toml`, `src/app/reducer.rs`, `src/workspace/ops.rs`
    - Steps:
      - Add explicit resize actions (horizontal/vertical +/-).
      - Add configurable keybinds + defaults (keep them Vim-ish and conflict-free).
      - Apply resize via `workspace::ops::resize_focused` in the reducer so behavior is shared between TUI + headless.
      - Add/extend a unit/integration test that asserts ratios actually change (no silent no-op), e.g. via deterministic layout rect sizes.
    - Verification (smallest signal): `cargo test -q` (workspace resize + reducer wiring)
    - Exit criteria: resize works via keybindings and is provably exercised by a programmatic test.
    - Rollback: revert commit.
* Verification (smallest signal):
  * `cargo test` runs and includes `tests/workspace_tree.rs` (pure, fast).
  * Manual: split/close/resize/focus works exactly as expected; no key leakage into the wrong pane.
* Exit criteria:
  * Arbitrary nested splits work and are stable.
  * Keymap config round-trips (change binding → behavior changes) without code edits.
* Rollback:
  * Revert commits; no external state created except local config.

## Phase 3 — Slack tokens + Socket Mode connection + sidebar conversation lists

* Goal:
  * Connect to a real Slack workspace and populate the left sidebar with channels + DMs, with a visible connection status.
* Work (microtasks):
  - [x] P3.T0 — Add Slack dependencies (`slack-morphism` + Socket Mode support)
    - File anchors: `Cargo.toml`
    - Steps:
      - Add `slack-morphism` (and any required runtime deps it needs for Socket Mode/Web API).
      - Keep the Slack dependency surface behind `src/slack/*` (no vendor types in UI).
      - If the toolchain/MSRV fights you, pin dependencies explicitly rather than carrying local RUSTFLAGS shims.
    - Verification (smallest signal): `cargo check`
    - Exit criteria: Slack crates build cleanly before wiring behavior.
    - Rollback: revert commit.
  - [x] P3.T1 — Implement token types + validation + redaction helpers
    - File anchors: `src/slack/tokens.rs` (new), `src/main.rs`, `config.example.toml`
    - Steps:
      - Source tokens from env vars (typically via a local `.env` in dev): `APP_TOKEN` (Socket Mode, `xapp-*`) and `BOT_TOKEN` (bot Web API, `xoxb-*`). Tokens are intentionally not part of the TOML config shape.
      - Validate token types (`xapp-*` for Socket Mode; `xoxb-*` for bot Web API; optional `xoxp-*`).
      - Ensure logs/debug output redact secrets by default.
    - Verification (smallest signal): invalid token fails loudly; valid token never appears in logs
    - Exit criteria: token leaks are hard to trigger accidentally.
    - Rollback: rotate tokens if leaked (should not happen); revert commit.
  - [x] P3.T2 — Define `SlackService` boundary + typed methods used by UI/effects
    - File anchors: `src/slack/service.rs` (new), `src/slack/mod.rs`
    - Steps:
      - Wrap `slack-morphism` so panes/state don’t depend on vendor types directly.
      - Start with: list conversations, fetch history (stubs allowed initially).
    - Verification (smallest signal): `cargo check`
    - Exit criteria: only `src/slack/*` touches slack-morphism types.
    - Rollback: revert commit.
  - [x] P3.T3 — Implement Socket Mode connect + ACK-fast loop
    - File anchors: `src/slack/socket_mode.rs` (new), `src/app/runtime.rs` (new), `src/app/app.rs`
    - Steps:
      - Connect and ACK events quickly (ACK fast, process async).
      - Feed events into an internal channel for the app runtime.
    - Verification (smallest signal): `cargo run` shows “connected” banner and increments an event counter
    - Exit criteria: events are received and ACKed reliably.
    - Rollback: revert commit.
  - [x] P3.T4 — Populate left sidebar with real channel + DM lists (and a Threads section shell)
    - File anchors: `src/app/sidebar.rs` (new), `src/app/state.rs`, `src/ui/sidebar.rs`, `src/slack/service.rs`
    - Steps:
      - Fetch conversations list; cache minimal fields (id, name, kind).
      - Render and allow keyboard selection (Vim-ish defaults: `j/k`, `Enter`).
      - Include a Threads section in the sidebar UI (it can be empty until Phase 5 populates it).
    - Verification (smallest signal): manual: sidebar shows real channel/DM lists
    - Exit criteria: sidebar is real Slack data (no fixtures).
    - Rollback: revert commit.
  - [x] P3.T5 — Fail-fast connection errors (explicit fatal error, no auto-recovery)
    - File anchors: `src/diagnostics/banner.rs`, `src/slack/socket_mode.rs`, `src/app/app.rs`
    - Steps:
      - Show connected status while healthy (header/status line).
      - On connection failure, render a fatal error (error chain + next action) and exit/crash (terminal restored).
    - Verification (smallest signal): disable network → fatal error screen with actionable message; app exits with terminal restored
    - Exit criteria: failures are never masked; when it breaks, it breaks loudly and immediately.
    - Rollback: revert commit.
  - [x] P3.T6 — Delete/cleanup: enforce “no Slack I/O on the render path”
    - File anchors: `src/app/effects.rs` (new), `src/app/app.rs`, `src/app/headless.rs`, `src/ui/*`
    - Steps:
      - Introduce an effects boundary for all Slack calls.
      - Keep UI draw functions pure (derive from state only).
    - Verification (smallest signal): `rg \"await|SlackService\" -n src/ui src/workspace` shows no Slack calls in UI/workspace rendering
    - Exit criteria: render/input loop is never blocked on network.
    - Rollback: n/a (architecture invariant).
* Verification (smallest signal):
  * Manual: tokens configured → `cargo run` shows real channel list and DM list.
* Exit criteria:
  * Sidebar is real and driven by Slack.
  * Socket Mode is connected and ACKs events (even if we don’t fully render them yet).
* Rollback:
  * Revert commits; rotate Slack tokens only if there was a leak.

## Phase 3.1 — Headless non-interactive driver mode (immediate priority)

* Goal:
  * Make `slack-rs` runnable and verifiable without an interactive TTY: start the app, drive actions via a script, and capture logs/output programmatically so an agent/CI can run end-to-end checks.
* Work (microtasks):
  - [x] P3.1.T1 — Define the headless “command surface” (SSOT = `Action`)
    - File anchors: `src/app/action.rs`, `docs/TERMINAL_DRIVEN_RUST_SLACK_TUI_WITH_VIM_KEYBINDINGS_2026-03-01.md` (this section)
    - Steps:
      - Ensure every user-visible behavior is expressible as an `Action` (or a small extension of it), not hidden in key-handling/UI code.
      - Document the minimum required headless commands for `v0.1.0` (split panes, focus movement, enter/leave compose, quit).
    - Verification (smallest signal): `cargo check`
    - Exit criteria: we have a stable “drive surface” for both keybindings and scripts.
    - Rollback: n/a (design).
  - [x] P3.1.T2 — Refactor the app loop so input + rendering are injectable
    - File anchors: `src/app/app.rs` (refactor), `src/ui/root.rs`
    - Steps:
      - Split `App::run()` into a core loop that can run with:
        - interactive backend (crossterm + real terminal)
        - headless backend (no raw mode; no alt screen)
      - Keep draw functions pure (state → frame), and keep network I/O off the loop as-is.
    - Verification (smallest signal): `cargo test -q`
    - Exit criteria: core loop can run without calling crossterm event APIs.
    - Rollback: revert commit.
  - [x] P3.1.T3 — Add `--headless` mode + scripted command runner
    - File anchors: `src/main.rs` (arg parsing), `src/app/headless.rs` (new)
    - Steps:
      - Implement `slack-rs --headless --script <path|->`:
        - reads newline-delimited commands (mapped to `Action`),
        - runs for a bounded number of ticks or until `quit`,
        - prints machine-readable output/log markers to stdout/stderr.
      - Do **not** require a TTY in this mode (no raw mode, no alt screen).
    - Verification (smallest signal): `cargo run -- --headless --script - <<<'quit'` exits 0
    - Exit criteria: we can start/stop the app non-interactively and observe output.
    - Rollback: revert commit.
  - [x] P3.1.T4 — Headless smoke script + integration test (spawn binary, assert output)
    - File anchors: `tests/headless_smoke.rs` (new)
    - Steps:
      - Add an integration test that spawns the built binary in `--headless` mode, feeds a short script, and asserts:
        - exit code success
        - expected log/output markers exist (e.g., “started”, “quit”, and at least one rendered frame marker).
    - Verification (smallest signal): `cargo test -q`
    - Exit criteria: we can verify end-to-end behavior without a human-operated terminal.
    - Rollback: revert commit.
  - [x] P3.1.T5 — Trap check: ensure interactive-only terminal code never runs in `--headless`
    - File anchors: `src/terminal/mod.rs`, `src/app/headless.rs`
    - Steps:
      - Add a single guard/invariant so `TerminalGuard::enter()` is never called in headless mode.
      - Ensure failures are explicit (no silent fallback to interactive).
    - Verification (smallest signal): `cargo test -q`
    - Exit criteria: headless mode is deterministic and doesn’t depend on environment quirks.
    - Rollback: revert commit.
  - [x] P3.1.T6 — Live Slack automation: headless “smoke” commands (Socket Mode + Web API)
    - File anchors: `src/app/headless.rs`, `src/slack/socket_mode.rs`, `src/slack/service.rs`, `tests/headless_live_slack.rs` (new)
    - Steps:
      - Extend `--headless --script` to support a minimal Slack live smoke sequence:
        - `slack_auth_test` (Web API) prints a structured summary.
        - `slack_socket_mode_smoke` connects, observes `hello`, then exits (ACKing envelopes if any).
      - Add a dedicated integration test for the live Slack smoke flow.
        - Keep it opt-in (requires tokens + explicit env var) to avoid accidental workspace mutation.
    - Verification (smallest signal): `SLACK_LIVE_TEST=1 cargo test -q -- --ignored`
    - Exit criteria: an agent can verify live Slack connectivity non-interactively.
    - Rollback: revert commit.
  - [x] P3.1.T7 — Headless Socket Mode listen (bounded) + apply live actions to state
    - File anchors: `src/slack/socket_mode.rs`, `src/app/headless.rs`, `tests/headless_live_slack_socket_mode_listen.rs` (new)
    - Steps:
      - Add a headless script command: `slack_socket_mode_listen <duration_ms>`.
      - It must connect, ACK envelopes fast, collect any translated actions (e.g. `SlackMessageReceived`), apply them to the reducer, then return.
      - Keep it opt-in test-only (no CI) and read-only (no workspace mutation).
    - Verification (smallest signal): `SLACK_LIVE_TEST=1 cargo test -q -- --ignored` (listen sees `hello`)
    - Exit criteria: headless automation can validate real-time connectivity and (when combined with send later) real-time timeline updates.
    - Rollback: revert commit.
  - [x] P3.1.T8 — Fail-fast Slack call timeouts (avoid hangs in interactive/headless)
    - File anchors: `src/app/effects.rs`, `src/app/app.rs`, `src/app/headless.rs`
    - Steps:
      - Add a single default timeout for Slack Web API effects, and surface a timeout as a fatal error.
      - Apply consistently in interactive `spawn_effects` and headless `SlackHarness`.
    - Verification (smallest signal): `cargo test -q` (compile/typecheck) + manual: disable network and confirm we exit with a fatal error (no hang).
    - Exit criteria: no Slack call can stall the UI/headless run indefinitely.
    - Rollback: revert commit.
  - [x] P3.1.T9 — Headless composer typing command (so send can be automated)
    - File anchors: `src/app/headless.rs`
    - Steps:
      - Add a headless command like `type <text>` that inserts text into the composer buffer (COMPOSE mode only).
      - Support basic escapes (`\\n` for newline) so multi-line compose can be scripted without a real terminal.
    - Verification (smallest signal): `cargo test -q`
    - Exit criteria: headless scripts can drive the write path without scraping key events.
    - Rollback: revert commit.
  - [x] P3.1.T10 — Live Slack automation: headless send-path test (explicit write opt-in)
    - File anchors: `tests/headless_live_slack_send_path.rs`
    - Steps:
      - Add an ignored live Slack test that uses headless mode to:
        - open a known conversation,
        - compose a unique message,
        - send it via the composer send action.
      - Gate it behind `SLACK_LIVE_TEST=1` *and* `SLACK_LIVE_TEST_ALLOW_WRITES=1` to prevent accidental workspace mutation.
    - Verification (smallest signal): `SLACK_LIVE_TEST=1 SLACK_LIVE_TEST_ALLOW_WRITES=1 SLACK_TEST_CONVERSATION_ID=... cargo test -q -- --ignored`
    - Exit criteria: we can validate the write path end-to-end non-interactively when we explicitly allow writes.
    - Rollback: delete test message manually (optional); revert commit.
* Verification (smallest signal):
  * `cargo test -q` runs the headless smoke test.
* Exit criteria:
  * We (and other agents) can run end-to-end checks without an interactive TTY.
* Rollback:
  * Revert commits (no external state).

## Phase 4 — Timeline panes + caching + real-time message updates (read path)

* Goal:
  * Selecting a channel/DM shows a timeline pane with recent messages, and new messages appear in near real time without a polling-first design.
* Work (microtasks):
  - [x] P4.T1 — Define the minimal local model + caches (messages/users/conversations)
    - File anchors: `src/model/mod.rs`, `src/model/conversation.rs`, `src/model/message.rs`, `src/model/user.rs`, `src/app/state.rs`, `src/app/timeline.rs`
    - Steps:
      - Define internal types that are stable and UI-friendly.
      - Add small caches keyed by Slack IDs (users, messages by conversation).
    - Verification (smallest signal): `cargo check`
    - Exit criteria: panes can render without touching slack-morphism types directly.
    - Rollback: revert commit.
  - [x] P4.T2 — Add timeline pane type and wire it into the workspace
    - File anchors: `src/workspace/tree.rs`, `src/ui/workspace.rs`, `src/app/reducer.rs`, `src/ui/sidebar.rs`
    - Steps:
      - Add a `PaneKind::Timeline { conversation }` kind.
      - Ensure selecting a conversation opens/updates the focused pane with that context.
    - Verification (smallest signal): manual: select sidebar conversation → timeline pane opens with correct title
    - Exit criteria: pane types exist and can be swapped/focused.
    - Rollback: revert commit.
  - [x] P4.T3 — Initial history fetch effect + cache fill (latest page)
    - File anchors: `src/app/effects.rs`, `src/slack/service.rs`, `src/app/reducer.rs`, `src/ui/workspace.rs`
    - Steps:
      - On conversation open, fetch recent messages and store in state.
    - Verification (smallest signal): headless: `sidebar_refresh` then `open <conversation_id>` yields a non-zero `focused_timeline_messages` in the JSON state output.
    - Exit criteria: timeline can show recent messages for a conversation the token can access.
    - Rollback: revert commit.
  - [x] P4.T3.1 — Cursor pagination to reach full history (load older repeatedly)
    - File anchors: `src/app/action.rs`, `src/app/reducer.rs`, `src/app/effects.rs`, `src/slack/service.rs`, `src/input/keymap.rs`, `src/app/headless.rs`, `tests/timeline_pagination.rs`, `tests/headless_live_slack_pagination.rs`
    - Steps:
      - Add a single “load older” action (`Action::TimelineLoadOlder`) that targets the *focused* timeline pane and uses `next_cursor` to fetch the next page.
      - Treat `next_cursor` as the source of truth for “has more” (cursor present → more history available).
      - Append older pages into the timeline cache without duplicating messages (dedup by `ts`).
      - Keep scroll/selection as Phase 5 work (we can paginate without it).
    - Verification (smallest signal):
      - Unit: `cargo test -q` (merge + effect emission)
      - Live (opt-in): `SLACK_LIVE_TEST=1 SLACK_TEST_CONVERSATION_ID=... cargo test -q -- --ignored` (headless pagination)
    - Exit criteria:
      - Full history is reachable on demand for a typical channel/DM (repeat load older until cursor is exhausted).
    - Rollback: revert commit.
  - [x] P4.T4 — Translate Socket Mode message events into reducer actions
    - File anchors: `src/slack/events.rs` (new), `src/slack/socket_mode.rs`, `src/app/action.rs`, `src/app/reducer.rs`, `tests/slack_events.rs`
    - Steps:
      - Handle “new message” events at minimum (ignore subtypes like edits/deletes for now).
      - Translate Socket Mode envelopes into internal actions and push them into the UI loop without blocking ACK.
      - Update the in-memory timeline cache incrementally (no polling-first refetch loops).
    - Verification (smallest signal):
      - Unit: `cargo test -q` (envelope translation)
      - Manual: send a new message in a loaded conversation; it appears in the timeline within seconds.
    - Exit criteria: real-time updates work without polling-first UX.
    - Rollback: revert commit.
  - [x] P4.T5 — Build a minimal renderer: mrkdwn tokens + code blocks + timestamps/authors
    - File anchors: `src/render/mrkdwn.rs` (new), `src/render/mod.rs`, `src/ui/workspace.rs`, `tests/mrkdwn_render.rs`
    - Steps:
      - Render plain text with author + timestamp.
      - Add minimal mrkdwn token handling (`<@U..>`, `<#C..>`, `<http..|..>`). Preserve code blocks verbatim (no lossy parsing).
    - Verification (smallest signal): `cargo test -q` (renderer unit tests) + manual: typical Slack messages are readable (not JSON dumps)
    - Exit criteria: reading is useful for real work.
    - Rollback: revert commit.
  - [x] P4.T6 — Delete/cleanup: remove any debug JSON rendering paths
    - File anchors: `src/ui/workspace.rs`, `src/render/*`
    - Steps:
      - Ensure the UI always renders via the renderer module (single path).
    - Verification (smallest signal): manual: no pane shows raw JSON by default
    - Exit criteria: consistent rendering across panes.
    - Rollback: revert commit.
* Verification (smallest signal):
  * Manual: select a channel; messages render; new messages appear without manual refresh.
* Exit criteria:
  * Channel/DM reading is useful for real work.
* Rollback:
  * Revert commits; no Slack state is mutated yet.

## Phase 5 — Threads + reactions + composing in context (write path)

* Goal:
  * Open threads side-by-side, reply in threads, react/unreact, and send messages to the focused pane’s context.
* Work (microtasks):
  - [x] P5.T0 — Populate the sidebar “Threads” section (open/recent threads) without hiding channels
    - File anchors: `src/app/sidebar.rs`, `src/app/reducer.rs`, `src/ui/sidebar.rs`, `src/model/thread.rs`, `src/app/headless.rs`
    - Steps:
      - When a thread pane is opened (or a thread is interacted with), add/update an entry in the sidebar Threads list.
      - Selecting a thread entry opens/focuses the thread pane; channels + DMs remain visible.
    - Verification (smallest signal):
      - Manual: open a thread → it appears in sidebar Threads; channels list is still visible
      - Headless: `open <conv>` → `thread` produces `sidebar_threads > 0` and `focused_pane_kind=THREAD`
    - Exit criteria: you can “see your threads” in the sidebar while still seeing channels/DMs.
    - Rollback: revert commit.
  - [x] P5.T1 — Add selection/focus model within timeline panes (message cursor)
    - File anchors: `src/app/pane_view.rs`, `src/app/state.rs`, `src/app/reducer.rs`, `src/input/keymap.rs`, `src/ui/workspace.rs`, `src/ui/sidebar.rs`, `src/app/headless.rs`, `tests/timeline_selection.rs`
    - Steps:
      - Add a per-pane “selected message ts” cursor (stable across pagination/new inserts) so there is always an explicit target for thread/reaction actions.
      - Add a minimal focus model so `j/k/gg/G` can mean “timeline selection” without breaking sidebar navigation (default: `Tab` toggles focus between sidebar/workspace).
      - Render the selected message highlight in the focused timeline pane using a stateful list so selection is scrollable.
      - Expose selection state in headless JSON output (`focused_timeline_selected_ts`) so scripts can verify behavior without a TTY.
    - Verification (smallest signal):
      - Unit: `cargo test -q` (selection behavior)
      - Headless: script `open <conv>` → `tab` → `timeline_select_prev/next` and observe `focused_timeline_selected_ts` changes line-by-line
      - Manual: selection moves and is highlighted; focused border indicates whether selection keys apply
    - Exit criteria: thread/reaction actions have a deterministic, scriptable target message.
    - Rollback: revert commit.
  - [x] P5.T2 — Implement thread pane open + fetch replies
    - File anchors: `src/workspace/tree.rs`, `src/app/action.rs`, `src/app/reducer.rs`, `src/app/effects.rs`, `src/slack/service.rs`, `src/ui/workspace.rs`, `src/input/keymap.rs`, `config.example.toml`, `src/app/headless.rs`, `tests/headless_live_slack_thread_reply.rs` (ignored)
    - Steps:
      - Open thread from selected message (binding configurable; default can be `Enter`).
      - Fetch replies and show them in a thread pane.
    - Verification (smallest signal):
      - Manual: open a thread; replies render; updates appear
      - Automation (opt-in): `SLACK_LIVE_TEST=1 SLACK_LIVE_TEST_ALLOW_WRITES=1 SLACK_TEST_CONVERSATION_ID=... cargo test -q -- --ignored` (thread open + reply test)
    - Exit criteria: thread view is usable side-by-side with the parent timeline.
    - Rollback: revert commit.
  - [x] P5.T3 — Implement send pipeline for focused timeline panes (channel/DM) with `Enter`/`Shift+Enter` semantics
    - File anchors: `src/app/reducer.rs`, `src/app/effects.rs`, `src/slack/service.rs`, `src/ui/composer.rs`, `src/app/headless.rs`, `tests/headless_live_slack_send_path.rs` (ignored)
    - Steps:
      - Determine target from focused pane (channel/DM).
      - `Enter` sends; `Shift+Enter` newline (configurable; alternate newline if terminal can’t distinguish).
      - Surface failures in banner (fail loud).
    - Verification (smallest signal):
      - Manual: send a channel message and DM message
      - Automation (opt-in): `SLACK_LIVE_TEST=1 SLACK_LIVE_TEST_ALLOW_WRITES=1 SLACK_TEST_CONVERSATION_ID=... cargo test -q -- --ignored`
    - Exit criteria: write path works for channel/DM messages.
    - Rollback: delete test messages if desired; revert commits.
  - [x] P5.T3.1 — Add thread reply sends (`thread_ts`) once thread panes exist
    - File anchors: `src/workspace/tree.rs`, `src/app/reducer.rs`, `src/app/effects.rs`, `src/slack/service.rs`, `src/app/headless.rs`, `tests/headless_live_slack_thread_reply.rs` (ignored)
    - Steps:
      - When focused pane is a thread, plumb `thread_ts` to `chat.postMessage`.
      - Add a live ignored headless test for thread replies (gated behind `SLACK_LIVE_TEST_ALLOW_WRITES=1`).
    - Verification (smallest signal): manual: reply in a thread; replies render and update
    - Exit criteria: thread replies work end-to-end.
    - Rollback: delete test replies if desired; revert commits.
  - [x] P5.T4 — Implement reactions add/remove + UI update
    - File anchors: `src/app/action.rs`, `src/app/prompt.rs`, `src/app/state.rs`, `src/app/app.rs`, `src/app/reducer.rs`, `src/app/effects.rs`, `src/slack/service.rs`, `src/ui/workspace.rs`, `src/ui/root.rs`, `src/input/keymap.rs`, `config.example.toml`, `src/app/headless.rs`, `tests/headless_live_slack_reaction_toggle.rs` (ignored)
    - Steps:
      - Keep message reactions in the internal message model so the UI can render counts.
      - Add a minimal emoji prompt (status-line prompt is fine) and toggle add/remove for the selected message.
      - Headless path: support `react <emoji>` so automation can test add/remove without a TTY.
    - Verification (smallest signal):
      - Manual: add reaction; remove reaction; counts update without restart
      - Automation (opt-in): `SLACK_LIVE_TEST=1 SLACK_LIVE_TEST_ALLOW_WRITES=1 SLACK_TEST_CONVERSATION_ID=... cargo test -q -- --ignored` (reaction toggle test)
    - Exit criteria: core “read → react → reply” loop works (at least for the bot user tied to `BOT_TOKEN`).
    - Rollback: remove test reactions if desired; revert commits.
  - [x] P5.T5 — Delete/cleanup: ensure all writes go through effects boundary
    - File anchors: `src/app/effects.rs`, `src/slack/service.rs`
    - Steps:
      - Remove any direct Slack write calls from panes/UI (single write path).
    - Verification (smallest signal): `rg \"chat\\.postMessage|reactions\\.|files\\.\" -n src/ui src/app` returns no direct calls
    - Exit criteria: no hidden writes on the render/input path.
    - Rollback: n/a.
* Verification (smallest signal):
  * Manual checklist: send message, open thread + reply, add/remove reaction, observe real-time updates.
* Exit criteria:
  * The core “Slack work loop” is functional in terminal.
* Rollback:
  * Revert commits; Slack state is only test messages/reactions.

## Phase 6 — Files (upload/download) + snippets + richer Block Kit rendering

* Goal:
  * Handle the remaining “Slack parity” items that matter day-to-day: files, snippets, and common Block Kit blocks.
* Work (microtasks):
  - [x] P6.T1 — Implement modern file upload (3-step / external upload) as a single helper
    - File anchors: `src/slack/files.rs`, `src/slack/service.rs`, `src/app/effects.rs`, `src/app/reducer.rs`, `src/app/action.rs`, `src/input/keymap.rs`, `config.example.toml`, `src/app/headless.rs`, `tests/headless_live_slack_file_upload_download.rs` (ignored)
    - Steps:
      - Implement Slack’s modern 3-step upload flow (avoid deprecated `files.upload`).
      - Keep file upload as a helper with a clean contract (path + conversation → result).
    - Verification (smallest signal):
      - Headless (opt-in): `SLACK_LIVE_TEST=1 SLACK_LIVE_TEST_ALLOW_WRITES=1 SLACK_TEST_CONVERSATION_ID=... cargo test -q -- --ignored` (file upload/download test)
      - Manual: upload a small file; see it appear in the channel
    - Exit criteria: file upload works end-to-end.
    - Rollback: delete uploaded files (optional); revert commits.
  - [x] P6.T2 — Implement authenticated file download (`url_private_download`) + destination prompt
    - File anchors: `src/slack/files.rs`, `src/slack/service.rs`, `src/app/action.rs`, `src/app/effects.rs`, `src/app/reducer.rs`, `src/app/prompt.rs`, `src/app/app.rs`, `src/ui/root.rs`, `src/app/headless.rs`
    - Steps:
      - Prompt for destination path (minimal status-line prompt; no file browser required initially).
      - Download bytes with auth and write atomically (temp file + rename).
    - Verification (smallest signal): manual: download a file and confirm bytes match
    - Exit criteria: downloads are reliable and don’t leave partial files behind.
    - Rollback: delete downloaded files; revert commits.
  - [x] P6.T3 — Render attachments/snippets in timelines with usable affordances
    - File anchors: `src/model/message.rs`, `src/render/attachments.rs`, `src/ui/workspace.rs`, `src/app/reducer.rs`
    - Steps:
      - Show filename and best-effort metadata (size when available) in the message body.
      - Show snippet preview (truncated) when available (`files.info`).
    - Verification (smallest signal): manual: snippet messages are readable and discoverable
    - Exit criteria: files/snippets don’t feel invisible in the TUI.
    - Rollback: revert commits.
  - [x] P6.T4 — Expand Block Kit rendering for common blocks (blocks-first)
    - File anchors: `src/render/blocks.rs`, `src/render/mrkdwn.rs`, `src/slack/service.rs`
    - Steps:
      - Support at least: `section`, `context`, `divider`.
      - Keep “blocks-first then mrkdwn fallback” as the only rendering policy (SSOT).
    - Verification (smallest signal): manual: Block Kit-heavy messages remain readable
    - Exit criteria: rendering is consistent across panes.
    - Rollback: revert commits.
  - [x] P6.T5 — Delete/cleanup: ensure deprecated file APIs are not used
    - File anchors: `src/slack/files.rs`
    - Steps:
      - Keep code/comments explicit: “no `files.upload`”.
    - Verification (smallest signal): `rg \"files\\.upload\" -n src` returns nothing
    - Exit criteria: we don’t accidentally ship retired API usage.
    - Rollback: n/a.
* Verification (smallest signal):
  * Manual: upload a small file, download it, and read a snippet message.
* Exit criteria:
  * File up/download works end-to-end and is usable.
* Rollback:
  * Revert commits; delete uploaded/downloaded files if desired.

## Phase 7 — Stability + polish + docs (internal tool posture)

* Goal:
  * Make it comfortable to live in while keeping the “fail-fast” posture: stable when correct, and brutally explicit when broken (personal tool, low support burden).
* Work (microtasks):
  - [x] P7.T1 — Write README + config example + Slack app setup steps
    - File anchors: `README.md` (new), `config.example.toml`
    - Steps:
      - Document Slack app creation + required scopes.
      - Document where config lives and how keybindings work.
      - Document known terminal quirks (Alt/Option, `Shift+Enter` variability).
    - Verification (smallest signal): fresh machine check: “could I set this up again in 10 minutes?”
    - Exit criteria: setup instructions are copy/paste-able and accurate.
    - Rollback: n/a.
  - [x] P7.T2 — Add minimal packaging guidance (personal tool posture)
    - File anchors: `README.md`
    - Steps:
      - Add `cargo install --path .` and release build notes.
    - Verification (smallest signal): `cargo install --path .` works locally
    - Exit criteria: reinstall is frictionless.
    - Rollback: n/a.
  - [ ] P7.T3 — Final acceptance checklist pass (Section 0.4) + polish fixes
    - File anchors: `docs/TERMINAL_DRIVEN_RUST_SLACK_TUI_WITH_VIM_KEYBINDINGS_2026-03-01.md` (checklist), `src/diagnostics/banner.rs`
    - Steps:
      - Run the Section 0.4 checklist end-to-end in your real workspace.
      - Fix only what blocks daily usability (avoid overbuild).
    - Verification (smallest signal): manual checklist passes with 0 panics
    - Exit criteria: you can use it daily without it feeling fragile.
    - Rollback: revert commits; Slack data remains in Slack.
* Verification (smallest signal):
  * Final manual acceptance checklist from Section 0.4 passes in your real workspace.
  * `cargo test` passes; `cargo clippy` is optional if it slows iteration early.
* Exit criteria:
  * Daily-driver usable; failures are diagnosable; setup is documented.
* Rollback:
  * Revert commits; config remains local; Slack data remains in Slack.
<!-- arch_skill:block:phase_plan:end -->

---

# 8) Verification Strategy (common-sense; non-blocking)

> Principle: avoid verification bureaucracy. Prefer the smallest existing signal. If sim/video/screenshot capture is flaky or slow, rely on targeted instrumentation + a short manual QA checklist and keep moving.
> Default: 1–3 checks total. Do not invent new harnesses/frameworks/scripts unless they already exist in-repo and are the cheapest guardrail.
> Default: keep UI/manual verification as a finalization checklist (don’t gate implementation).
> Default: do NOT create “proof” tests that assert deletions, visual constants, or doc inventories. Prefer compile/typecheck + behavior-level assertions only when they buy confidence.
> Also: document any new tricky invariants/gotchas in code comments at the SSOT/contract boundary so future refactors don’t break the pattern.

## 8.1 Unit tests (contracts)

* Invariants we unit-lock (pure, fast, deterministic):
  * Split-tree correctness: `tests/workspace_tree.rs` (splits, focus moves, close behavior).
  * Keymap determinism: `src/input/keymap.rs` + `src/input/normalize.rs` (expand with focused unit tests as mappings grow; avoid PTY tests here).
  * Reducer purity + state transitions: `src/app/reducer.rs` (compose/send/newline semantics, pane ops; add unit tests when we add non-trivial transitions beyond pane ops).
  * Token validation + redaction: `src/slack/tokens.rs` (optional unit tests; never assert token values, only prefix validation and redaction).

## 8.2 Integration tests (flows)

* Critical flows (non-interactive; preferred over manual):
  * Headless driver smoke: `tests/headless_smoke.rs` (spawns the binary; drives pane ops; asserts JSON markers).
  * Live Slack connectivity smoke: `tests/headless_live_slack.rs` (ignored; requires `SLACK_LIVE_TEST=1` + `APP_TOKEN`/`BOT_TOKEN`; asserts Web API auth + Socket Mode hello).
  * As each Slack feature lands, add at least one new headless script-driven integration test that exercises it end-to-end (still without a TTY).
* Failure injection (fail-fast; no retries):
  * Invalid/missing tokens → explicit fatal error chain and non-zero exit.
  * Network disabled → explicit fatal error chain and non-zero exit.
  * Socket Mode `disconnect` message → explicit fatal error chain and non-zero exit.

## 8.3 E2E / device tests (realistic)

* Scenarios (live Slack; still no humans; opt-in + safe):
  * Connectivity:
    * `slack_auth_test` (Web API) returns team/user.
    * `slack_socket_mode_smoke` observes `hello`.
  * Read path (as we implement timelines):
    * list sidebar conversations → open channel/DM → fetch recent history → paginate older.
    * open thread view → fetch replies + context.
  * Write path (bounded; dedicated channel/thread; explicit cleanup where possible):
    * post a message with a unique test prefix → verify it appears in history (and delete if permissions allow).
    * add/remove a reaction on the test message.
    * upload a small file and verify metadata + download works.
* Evidence / artifacts (automation-friendly):
  * Primary: JSON line output from `--headless --script` (stable keys; assert markers and counts).
  * Optional: Ratatui `TestBackend` snapshots for key UI states (sidebar sections visible; focused pane borders; composer mode).
* Live Slack E2E config (explicit; no hidden defaults):
  * Required: `APP_TOKEN`, `BOT_TOKEN` (already; env-only).
  * Gate: `SLACK_LIVE_TEST=1` to run live tests.
  * Planned (as needed): `SLACK_TEST_CHANNEL_ID`, `SLACK_TEST_USER_ID`, `SLACK_TEST_FILE_PATH` (avoid guessing; keep tests explicit and safe).

---

# 9) Rollout / Ops / Telemetry

## 9.1 Rollout plan

* Flags / gradual rollout (only if needed; avoid long-lived dual paths):
* Rollback plan (preferred over runtime shims): revert commit / kill-switch / disable new path

## 9.2 Telemetry changes

* New events:
* New properties:
* Dashboards / alerting:

## 9.3 Operational runbook

* Debug checklist:
* Common failure modes + fixes:

---

# 10) Decision Log (append-only)

## 2026-03-01 — Draft North Star + scope created

* Context: New repo; initial requirements captured from the project blurb.
* Options: N/A (bootstrapping).
* Decision: Draft North Star (Section 0) and keep `fallback_policy: forbidden` until explicitly approved.
* Consequences: Next step is confirming the North Star before doing any research or architecture deep dives.
* Follow-ups: Confirm “yes/no”; if “yes”, activate the doc (`status: active`) and proceed to research grounding + architecture deep dive.

## 2026-03-01 — External research: choose defaults + bake in Slack platform constraints

* Context: We need a stack that accelerates building a split-pane, Vim-keybinding-heavy Slack client, while staying aligned with modern Slack APIs and constraints.
* Options:
  * TUI: Ratatui vs Cursive vs higher-level frameworks (tui-realm) vs declarative approaches (iocraft).
  * Keymaps: build our own vs use a key sequence dispatcher (`keybinds`) + helper crates (`crokey`).
  * Slack integration: full-stack crate (`slack-morphism`) vs legacy RTM crates vs generated Web API clients.
  * Real-time: Socket Mode/Events API vs RTM.
  * Files: deprecated `files.upload` vs modern 3-step external upload.
* Decision:
  * Default to `ratatui` + `crossterm` with our own split-tree workspace and explicit focus routing.
  * Default to `keybinds` for mode-aware action dispatch (Vim-like sequences) and `tui-textarea` for the composer (evaluate `edtui` later if we want deeper Vim editor behavior).
  * Default to `slack-morphism` and implement Socket Mode early to reduce polling and rate-limit risk.
  * Implement the modern file upload flow (do not build on `files.upload`).
* Consequences:
  * The plan should sequence Socket Mode earlier than “nice-to-have real-time” because it materially reduces API polling pressure.
  * Upload/download support must be designed around the modern APIs (multi-step upload + authenticated private downloads).
* Follow-ups:
  * Confirm distribution model (internal-only vs broader distribution) to calibrate rate-limit risk and auth UX.

## 2026-03-01 — Distribution model confirmed (personal/internal tool)

* Context: This is a tool for a single user (aelaguiz). It may be open sourced, but there is no intent to distribute broadly or provide heavy support.
* Options:
  * “Internal-only” posture (manual setup, self-managed Slack app, minimal installer work).
  * “Productized” posture (packaging, updates, broad terminal support guarantees, Marketplace considerations).
* Decision:
  * Build with best practices and a clean architecture, but optimize for “internal-only” ergonomics:
    - Manual Slack app setup is acceptable (document it clearly).
    - Packaging can be lightweight (ship a single binary + config file; no auto-updater required).
    - Assume non-Marketplace constraints and design around rate limits (Socket Mode + caching remains the default).
* Consequences:
  * We should invest early in “debuggability” (explicit fatal errors + good logs). Diagnostics screens are optional and should be added only if terminal quirks block progress.
  * Keybinding defaults should work in common terminals, but we can document terminal-specific quirks rather than trying to support every edge case perfectly.
* Follow-ups:
  * None (captured and resolved).

## 2026-03-01 — Composer semantics (Enter sends; Shift+Enter newline)

* Context: For a Slack-like workflow inside the TUI, `Enter` should send by default, but we must still support multi-line composition without friction.
* Options:
  * `Enter` sends; `Shift+Enter` newline (Slack-like, but terminal support varies).
  * `Enter` newline; send via modified key (Vim/editor-like; safer for multi-line).
* Decision:
  * `Enter` sends the composed message.
  * `Shift+Enter` inserts a newline in the composer (default).
  * Because some terminals can’t distinguish `Shift+Enter` from `Enter`, keep this configurable and recommend an alternate newline binding (e.g., `Ctrl+Enter`) when needed.
* Consequences:
  * We should treat “send vs newline” as a first-class, user-configurable setting. If terminal quirks block progress, add a diagnostics surface later — don’t build it up front.
  * We should enable enhanced keyboard reporting (when available) to maximize our ability to distinguish modified keys like `Shift+Enter`.
* Follow-ups:
  * None (captured and resolved; bindings remain configurable).

## 2026-03-01 — Fail-fast iteration posture + de-scope “nice-to-have” UI

* Context: This is a personal tool and the primary goal is fast iteration. Masking bugs via recovery/overlays/persistence slows iteration and can hide correctness issues.
* Options:
  * “Resilient client” posture: reconnect loops, offline banners, keep running in degraded mode.
  * “Fail-fast” posture: crash/exit immediately with an explicit error chain after restoring terminal state.
* Decision:
  * Default to **fail-fast** during `v0.1.0`: on Slack/Socket Mode/protocol/model errors that would compromise correctness, render an explicit fatal error (error chain + next steps), restore the terminal, and exit/crash. Do not auto-recover.
  * De-scope for now (add only if we miss them or hit blocking issues):
    - which-key/cheatsheet UI
    - key-event diagnostics UI
    - persistence of last conversation or workspace layout
* Consequences:
  * Connection “recovery” is not a goal; we prefer obvious failure + fixes.
  * The sidebar must always show Channels + DMs + Threads simultaneously (threads view/pane must not hide channels).
  * Full history must be reachable via pagination (lazy-loaded), rather than relying on persistence or partial history caps.
* Follow-ups:
  * If fail-fast becomes too annoying for daily use, we can add a deliberate “resilient mode” later — but only as an explicit decision (no hidden shims).

## 2026-03-02 — Clarify sidebar threads semantics (recent/open) + drop sidebar search UI

* Context: The core requirement for the sidebar is “Channels + DMs + Threads visible simultaneously”. We don’t need to overbuild a global Threads/Search experience for `v0.1.0`.
* Options:
  * Threads list = global “all my threads” view (harder; may require additional Slack APIs/search semantics).
  * Threads list = “recent/open threads in this client” (simple; matches day-to-day navigation needs).
  * Sidebar search/jump UI vs no search UI for `v0.1.0`.
* Decision:
  * Implement Threads as a “recent/open threads” list in the sidebar (SSOT managed by the client as you open threads, updated by events where possible).
  * Do not implement sidebar search/jump UI for `v0.1.0`.
* Consequences:
  * We preserve the core UX without scope creep; the split-pane workspace is still the primary differentiator.
  * If we later want “global threads” or search, we’ll add them deliberately as a new phase with Slack API evidence.
* Follow-ups:
  * None.

## 2026-03-02 — “Rip through” execution posture (stop only for real blockers)

* Context: This is a personal tool; the fastest path is to implement aggressively and iterate on failures, not to pause for repeated confirmations or speculative scope-proofing.
* Options:
  * Conservative posture: pause on open questions, build diagnostic/help UIs up front, avoid taking defaults.
  * Aggressive posture: pick idiomatic defaults, build the next slice, and fail fast with explicit errors when something breaks.
* Decision:
  * Adopt the aggressive posture: keep moving through phases; treat open questions as non-blocking; avoid “help UIs” unless we hit a real input/API blocker.
  * Still enforce strict invariants: no fallbacks, no secret leaks, and fail-fast error surfacing.
* Consequences:
  * Faster iteration (especially in Phase 3+ where Slack API friction will show up).
  * When something breaks, it breaks loudly and early (by design).
* Follow-ups:
  * None.

## 2026-03-02 — Socket Mode: custom websocket loop (no auto-reconnect)

* Context: We need near real-time updates, but we also want a strict fail-fast posture (no hidden reconnect loops that mask problems).
* Options:
  * Use slack-morphism’s Socket Mode manager/listener (simpler, but its default posture is to keep the session alive).
  * Implement a minimal custom Socket Mode websocket loop (connect → read → ACK → forward events) and crash on disconnect/error.
* Decision:
  * Implement our own Socket Mode websocket loop using `tokio-tungstenite`.
  * ACK envelopes immediately and forward lightweight events into the UI runtime channel.
  * On websocket close/disconnect/protocol errors: raise a fatal error and exit (no reconnect).
* Consequences:
  * We own the “ACK fast” behavior and any minimal parsing/dispatch we need.
  * This keeps the “no masked bugs / no auto-recovery” invariant honest.
* Follow-ups:
  * Once timelines render real data, decide which event types we must support first (new messages, edits, deletes).

## 2026-03-02 — Dependency pin: `serde_json = 1.0.147` (avoid toolchain break)

* Context: Newer `serde_json` pulls `zmij` builds that assume `std::hint::select_unpredictable` exists for `rustc 1.88+`. Our current toolchain reports `1.88.0-nightly` but does not provide that symbol, which breaks builds.
* Options:
  * Upgrade/switch toolchain to one that matches the dependency’s assumptions.
  * Carry local `RUSTFLAGS`/cfg shims to force fallback paths in transitive deps.
  * Pin `serde_json` to a compatible version.
* Decision:
  * Pin `serde_json` to `1.0.147` and keep moving; avoid local build-flag shims.
* Consequences:
  * We trade “latest serde_json” for deterministic builds on this toolchain.
* Follow-ups:
  * When we intentionally update toolchain, re-evaluate whether the pin can be removed.

## 2026-03-02 — Non-interactive testability is a North Star requirement

* Context: Agents (and CI) can’t reliably drive an interactive full-screen TUI. We need to be able to start the app, drive actions, and capture output/logs programmatically so end-to-end verification is possible without a human at the keyboard.
* Options:
  * Keep TUI-only and rely on manual testing (slow; blocks agent execution).
  * Add a headless mode that runs without a TTY, accepts scripted commands, and emits machine-readable output/log markers.
* Decision:
  * Make headless non-interactive mode an explicit deliverable and treat it as an invariant: every core action must be drivable via `Action` (keys are just one frontend).
  * Insert an immediate phase (Phase 3.1) to implement `--headless --script …` plus a smoke integration test.
* Consequences:
  * We can run end-to-end checks from automation and keep moving faster.
  * It nudges the architecture toward clean separation: core state/actions vs I/O frontends.
* Follow-ups:
  * Once timelines exist, extend the headless command surface to cover “open conversation”, “scroll/paginate”, and “send message” (still without requiring a real terminal).

## 2026-03-02 — Socket Mode TLS: enable `tokio-tungstenite` rustls support

* Context: Slack Socket Mode uses `wss://...` URLs, so the websocket client must be compiled with TLS support. Our initial `tokio-tungstenite` feature selection (`rustls-native-certs` only) compiled but failed at runtime with “TLS support not compiled in”.
* Options:
  * Use `native-tls` (platform TLS) for websockets.
  * Use rustls via `rustls-tls-native-roots` for websockets.
* Decision:
  * Use `tokio-tungstenite` with `rustls-tls-native-roots` so Socket Mode works out of the box for a personal tool.
* Consequences:
  * Socket Mode connectivity checks (and the UI Socket Mode loop) work against real Slack workspaces.
  * Slightly larger dependency surface (rustls stack), but consistent with our existing `hyper-rustls` usage via slack-morphism.
* Follow-ups:
  * None.
