use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use keybinds::{Key, KeyInput, Mods};

pub fn to_key_input(event: KeyEvent) -> KeyInput {
    // Treat key releases as ignored inputs. This keeps the dispatcher "press-driven" by default.
    if event.kind == KeyEventKind::Release {
        return KeyInput::new(Key::Ignored, Mods::NONE);
    }

    if event.code == KeyCode::BackTab {
        return KeyInput::new(Key::Tab, mods_from_crossterm(event.modifiers) | Mods::SHIFT);
    }

    KeyInput::new(
        key_from_crossterm(event.code),
        mods_from_crossterm(event.modifiers),
    )
}

fn mods_from_crossterm(mods: KeyModifiers) -> Mods {
    let mut out = Mods::NONE;

    if mods.contains(KeyModifiers::CONTROL) {
        out |= Mods::CTRL;
    }
    if mods.intersects(KeyModifiers::ALT | KeyModifiers::META) {
        out |= Mods::ALT;
    }
    if mods.contains(KeyModifiers::SUPER) {
        out |= Mods::SUPER;
    }
    if mods.contains(KeyModifiers::SHIFT) {
        out |= Mods::SHIFT;
    }

    out
}

fn key_from_crossterm(code: KeyCode) -> Key {
    match code {
        KeyCode::Char(c) => Key::Char(c),
        KeyCode::Enter => Key::Enter,
        KeyCode::Esc => Key::Esc,
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Delete => Key::Delete,
        KeyCode::Tab => Key::Tab,
        KeyCode::Insert => Key::Insert,
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        KeyCode::PageUp => Key::PageUp,
        KeyCode::PageDown => Key::PageDown,
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Left => Key::Left,
        KeyCode::Right => Key::Right,

        KeyCode::F(1) => Key::F1,
        KeyCode::F(2) => Key::F2,
        KeyCode::F(3) => Key::F3,
        KeyCode::F(4) => Key::F4,
        KeyCode::F(5) => Key::F5,
        KeyCode::F(6) => Key::F6,
        KeyCode::F(7) => Key::F7,
        KeyCode::F(8) => Key::F8,
        KeyCode::F(9) => Key::F9,
        KeyCode::F(10) => Key::F10,
        KeyCode::F(11) => Key::F11,
        KeyCode::F(12) => Key::F12,
        KeyCode::F(13) => Key::F13,
        KeyCode::F(14) => Key::F14,
        KeyCode::F(15) => Key::F15,
        KeyCode::F(16) => Key::F16,
        KeyCode::F(17) => Key::F17,
        KeyCode::F(18) => Key::F18,
        KeyCode::F(19) => Key::F19,
        KeyCode::F(20) => Key::F20,
        KeyCode::F(21) => Key::F21,
        KeyCode::F(22) => Key::F22,
        KeyCode::F(23) => Key::F23,
        KeyCode::F(24) => Key::F24,
        KeyCode::F(25) => Key::F25,
        KeyCode::F(26) => Key::F26,
        KeyCode::F(27) => Key::F27,
        KeyCode::F(28) => Key::F28,
        KeyCode::F(29) => Key::F29,
        KeyCode::F(30) => Key::F30,
        KeyCode::F(31) => Key::F31,
        KeyCode::F(32) => Key::F32,
        KeyCode::F(33) => Key::F33,
        KeyCode::F(34) => Key::F34,
        KeyCode::F(35) => Key::F35,

        KeyCode::BackTab => Key::Unidentified, // handled above
        KeyCode::Null => Key::Ignored,
        _ => Key::Unidentified,
    }
}
