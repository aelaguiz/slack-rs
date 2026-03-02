use std::io;
use std::time::Duration;

use anyhow::Context as _;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::app::action::Action;
use crate::app::config;
use crate::app::effects;
use crate::app::effects::Effect;
use crate::app::prompt::PromptKind;
use crate::app::runtime::{AppEvent, AppRuntime};
use crate::app::state::AppState;
use crate::input::keymap::KeymapMode;
use crate::slack::service::SlackService;
use crate::slack::socket_mode;
use crate::slack::tokens::SlackTokens;
use crate::terminal::TerminalGuard;
use crate::ui;

pub struct App;

impl App {
    pub fn run() -> anyhow::Result<()> {
        crate::diagnostics::log::init_tracing();

        let loaded_config = config::load().context("load app config")?;

        let _terminal_guard =
            TerminalGuard::enter().context("failed to enter terminal raw mode")?;

        let tokens = SlackTokens::from_env().context("load Slack tokens from env")?;
        let slack = SlackService::new(tokens).context("init Slack service")?;

        let (mut runtime, tx) = AppRuntime::new().context("init async runtime")?;
        {
            let slack = slack.clone();
            let tx = tx.clone();
            runtime.spawn(async move {
                let result = socket_mode::run_socket_mode(slack, tx.clone()).await;
                if let Err(err) = result {
                    // Fail-fast: surface the error to the UI loop, then exit.
                    let _ = tx.send(AppEvent::Fatal(err)).await;
                }
            });
        }

        let mut keymap = crate::input::keymap::KeyDispatcher::new(&loaded_config.config)
            .with_context(|| match &loaded_config.path {
                Some(path) => format!("init key dispatcher (config: {})", path.display()),
                None => "init key dispatcher (defaults)".to_string(),
            })?;

        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).context("failed to create terminal backend")?;
        terminal.clear().context("terminal clear failed")?;
        terminal
            .hide_cursor()
            .context("terminal hide_cursor failed")?;

        let mut state = AppState {
            shift_enter_confirmed: !needs_shift_enter_confirmation(&loaded_config.config),
            ..Default::default()
        };

        // Kick off the first sidebar load immediately (effects run off the render path).
        let initial = crate::app::reducer::apply_action(&mut state, Action::SidebarRefresh)
            .context("dispatch initial sidebar refresh")?;
        spawn_effects(&runtime, &tx, &slack, initial.effects);

        let mut cursor_visible = false;
        loop {
            let mut should_quit = false;
            while let Some(ev) = runtime.try_recv() {
                match ev {
                    AppEvent::SlackConnected => {
                        state.socket_mode_connected = true;
                    }
                    AppEvent::SlackEventReceived => {
                        state.socket_mode_events = state.socket_mode_events.saturating_add(1);
                    }
                    AppEvent::Action(action) => {
                        let outcome = crate::app::reducer::apply_action(&mut state, action)
                            .context("apply async action")?;
                        spawn_effects(&runtime, &tx, &slack, outcome.effects);
                        if outcome.should_quit {
                            should_quit = true;
                            break;
                        }
                    }
                    AppEvent::Fatal(err) => {
                        return Err(err);
                    }
                }
            }
            if should_quit {
                break;
            }

            if state.mode == KeymapMode::Compose && !cursor_visible {
                terminal
                    .show_cursor()
                    .context("terminal show_cursor failed")?;
                cursor_visible = true;
            } else if state.mode == KeymapMode::Normal && cursor_visible {
                terminal
                    .hide_cursor()
                    .context("terminal hide_cursor failed")?;
                cursor_visible = false;
            }

            terminal
                .draw(|frame| ui::root::draw_root(frame, &state))
                .context("ui draw failed")?;

            if event::poll(Duration::from_millis(50)).context("terminal event poll failed")? {
                match event::read().context("terminal event read failed")? {
                    Event::Key(key)
                        if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) =>
                    {
                        if state.prompt.is_some() {
                            // Prompt precedence: prompt input → (optional) prompt confirm/cancel → rest.
                            //
                            // We intentionally keep prompt UX minimal and fail-fast: prompts are only
                            // used when an action needs a parameter (e.g. emoji name).
                            let mut action_to_dispatch: Option<Action> = None;
                            if key.code == KeyCode::Char('c')
                                && key.modifiers.contains(KeyModifiers::CONTROL)
                            {
                                action_to_dispatch = Some(Action::Quit);
                            } else if matches!(key.code, KeyCode::Esc) {
                                state.prompt = None;
                            } else if matches!(key.code, KeyCode::Enter) {
                                let prompt =
                                    state.prompt.as_ref().expect("prompt exists by outer if");
                                match prompt.kind {
                                    PromptKind::ReactionEmoji => {
                                        let emoji = prompt.value();
                                        action_to_dispatch = Some(Action::ReactionToggle { emoji });
                                    }
                                    PromptKind::FileUploadPath => {
                                        let path = prompt.value();
                                        action_to_dispatch = Some(Action::FileUploadStart { path });
                                    }
                                    PromptKind::FileDownloadPath => {
                                        let dest_path = prompt.value();
                                        action_to_dispatch =
                                            Some(Action::FileDownloadStart { dest_path });
                                    }
                                }
                            } else if let Some(prompt) = state.prompt.as_mut() {
                                let _modified = prompt.input.input(key);
                            }

                            if let Some(action) = action_to_dispatch {
                                let outcome =
                                    crate::app::reducer::apply_action(&mut state, action)?;
                                spawn_effects(&runtime, &tx, &slack, outcome.effects);
                                if outcome.should_quit {
                                    break;
                                }
                            }
                        } else if let Some(action) =
                            keymap.dispatch(state.mode, state.focus_area, key)
                        {
                            let outcome = crate::app::reducer::apply_action(&mut state, action)?;
                            spawn_effects(&runtime, &tx, &slack, outcome.effects);
                            if outcome.should_quit {
                                break;
                            }
                        } else if state.mode == KeymapMode::Compose {
                            // Text input never goes through the global keymap.
                            let _modified = state.composer.input(key);
                        }

                        state.last_key = Some(format!("{key:?}"));
                    }
                    Event::Resize(_, _) => {
                        // Next draw iteration will adapt automatically.
                    }
                    _ => {}
                }
            }
        }

        // Ensure cursor state is restored before leaving alt screen.
        terminal
            .show_cursor()
            .context("terminal show_cursor failed")?;

        Ok(())
    }
}

fn spawn_effects(
    runtime: &AppRuntime,
    tx: &tokio::sync::mpsc::Sender<AppEvent>,
    slack: &SlackService,
    effects: Vec<Effect>,
) {
    for effect in effects {
        let slack = slack.clone();
        let tx = tx.clone();
        runtime.spawn(async move {
            let effect_debug = format!("{effect:?}");
            let timeout = effects::timeout_for_effect(&effect);

            let result = tokio::time::timeout(timeout, effects::execute(effect, slack)).await;
            match result {
                Ok(Ok(actions)) => {
                    for action in actions {
                        let _ = tx.send(AppEvent::Action(action)).await;
                    }
                }
                Ok(Err(err)) => {
                    let _ = tx.send(AppEvent::Fatal(err)).await;
                }
                Err(_) => {
                    let _ = tx
                        .send(AppEvent::Fatal(anyhow::anyhow!(
                            "Slack effect timed out after {timeout:?}: {effect_debug}"
                        )))
                        .await;
                }
            }
        });
    }
}

fn needs_shift_enter_confirmation(config: &config::AppConfig) -> bool {
    let kb = &config.keybinds;

    let send_bindings = bindings_or_default(kb.composer_send.as_ref(), &["Enter"]);
    let newline_bindings = bindings_or_default(kb.composer_newline.as_ref(), &["Shift+Enter"]);

    // The only truly-dangerous ambiguity we care about here is:
    // - send is bound to Enter
    // - newline is bound to Shift+Enter
    //
    // If the terminal can't report Shift+Enter distinctly, the user might accidentally send when
    // they meant newline. We fail fast by requiring Shift+Enter be observed once before allowing
    // Enter-to-send.
    contains_binding(&send_bindings, "Enter") && contains_binding(&newline_bindings, "Shift+Enter")
}

fn bindings_or_default(configured: Option<&Vec<String>>, defaults: &[&str]) -> Vec<String> {
    match configured {
        Some(v) => v.clone(),
        None => defaults.iter().map(|s| s.to_string()).collect(),
    }
}

fn contains_binding(bindings: &[String], needle: &str) -> bool {
    let needle = normalize_seq(needle);
    bindings
        .iter()
        .map(|s| normalize_seq(s))
        .any(|s| s == needle)
}

fn normalize_seq(raw: &str) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}
