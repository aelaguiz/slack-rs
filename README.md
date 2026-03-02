# slack-rs

Terminal-first Slack client in Rust with Vim-style configurable keybindings, a left sidebar (Channels + DMs + Threads), and an arbitrarily splittable right-side workspace.

This is a **personal tool**: it is intentionally **fail-fast**. If Slack I/O, parsing, or an invariant breaks, it will restore the terminal and exit with a loud error chain (set `RUST_BACKTRACE=1` when debugging).

## Features (current)

- Sidebar: Channels, DMs, Threads (recent/opened in this client)
- Workspace: arbitrary nested splits (Vim `Ctrl+w` style)
- Timelines + threads, near real-time updates via Socket Mode
- Composer: `Enter` to send, `Shift+Enter` for newline (configurable)
- Reactions: toggle emoji reactions on selected message
- Files:
  - Upload via Slack’s modern external upload flow
  - Download via `url_private_download` (authenticated)
  - Snippet preview (best-effort) via `files.info`
- Headless automation: `--headless --script <path|->` runs without a TTY and prints JSON lines

## Setup

### 1) Create a Slack app

- Enable **Socket Mode**
- Create an **App-Level Token** with `connections:write` → this is your `APP_TOKEN` (starts with `xapp-`)
- Install the app to your workspace to get a **Bot Token** → this is your `BOT_TOKEN` (starts with `xoxb-`)

Bot scopes you will likely need (not exhaustive):

- Read: `channels:read`, `groups:read`, `im:read`, `mpim:read`, `users:read`
- History: `channels:history`, `groups:history`, `im:history`, `mpim:history`
- Write: `chat:write`, `reactions:write`, `files:write`, `files:read`

If Slack returns a scope error, `slack-rs` will fail loudly and print the missing scope in the error chain.

### 2) Put tokens in `.env`

This repo expects `APP_TOKEN` and `BOT_TOKEN` to come from the environment (often via a local `.env`).

Example `.env` (do **not** commit):

```bash
APP_TOKEN="xapp-..."
BOT_TOKEN="xoxb-..."
```

## Running

Interactive:

```bash
cargo run
```

Install (optional):

```bash
cargo install --path .
```

Headless (non-interactive):

```bash
cargo run -- --headless --script -
```

Example headless script:

```text
# open a conversation and load history
open C12345678

# split the workspace and move focus
split_vertical
focus_right

# upload then download the selected file
upload ./hello.txt
download ./downloaded.txt

quit
```

Headless outputs newline-delimited JSON. Each script line produces an `"event":"state"` record after effects settle.

## Keybind config

Keybinds are loaded from:

- `./config.toml` (local dev override; gitignored), then
- OS config dir via `directories::ProjectDirs` (see `src/app/config.rs`)

Start from `config.example.toml`.

## Tests

Fast unit tests:

```bash
cargo test -q
```

Live Slack integration tests are **ignored by default** and require explicit opt-in:

```bash
SLACK_LIVE_TEST=1 SLACK_LIVE_TEST_ALLOW_WRITES=1 SLACK_TEST_CONVERSATION_ID=C123... \
  cargo test -q -- --ignored
```

## Notes / philosophy

- No GUI. No mouse.
- No reconnect/retry loops intended to “hide” bugs (fail-fast posture).
- Network I/O never happens on the UI render path (all Slack calls happen via effects).
