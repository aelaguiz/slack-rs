use std::io;
use std::panic;
use std::sync::atomic::{AtomicBool, Ordering};

use crossterm::cursor;
use crossterm::event::{
    KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::execute;
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};

static TERMINAL_ACTIVE: AtomicBool = AtomicBool::new(false);
static HEADLESS_MODE: AtomicBool = AtomicBool::new(false);

/// Mark the current process as running in headless mode.
///
/// This is a trap-check invariant: headless runs must never enter raw mode or the alternate screen.
pub fn mark_headless_mode() {
    HEADLESS_MODE.store(true, Ordering::SeqCst);
}

pub struct TerminalGuard {
    _private: (),
}

impl TerminalGuard {
    pub fn enter() -> anyhow::Result<Self> {
        if HEADLESS_MODE.load(Ordering::SeqCst) {
            anyhow::bail!(
                "TerminalGuard::enter() called while headless mode is active (bug: headless must not touch terminal raw mode)"
            );
        }

        let mut stdout = io::stdout();

        terminal::enable_raw_mode()?;
        TERMINAL_ACTIVE.store(true, Ordering::SeqCst);

        if let Err(err) = execute!(
            stdout,
            EnterAlternateScreen,
            cursor::Hide,
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
        ) {
            restore_best_effort();
            return Err(err.into());
        }

        install_panic_hook();

        Ok(Self { _private: () })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore_best_effort();
    }
}

pub fn restore_best_effort() {
    if !TERMINAL_ACTIVE.swap(false, Ordering::SeqCst) {
        return;
    }

    let mut stdout = io::stdout();
    let _ = execute!(
        stdout,
        PopKeyboardEnhancementFlags,
        cursor::Show,
        LeaveAlternateScreen
    );
    let _ = terminal::disable_raw_mode();
}

fn install_panic_hook() {
    static INSTALLED: AtomicBool = AtomicBool::new(false);
    if INSTALLED.swap(true, Ordering::SeqCst) {
        return;
    }

    let prev = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        restore_best_effort();
        prev(info);
    }));
}
