use std::sync::Mutex;

use anyhow::{Context, Result};

use crate::event::{InputAction, KeyEvent, MouseEvent};

/// Platform-specific input injection API.
///
/// The concrete backend wraps an `enigo::Enigo` behind a mutex, which restores
/// `Send + Sync` so the service can be shared across async tasks.
pub trait InputInjector: Send + Sync + std::fmt::Debug {
    /// Inject a single input action into the local system.
    fn inject(&self, action: &InputAction) -> Result<()>;
    fn available(&self) -> bool { true }
}

// ---------------------------------------------------------------------------
// enigo backend (macOS / Windows / Linux)
// ---------------------------------------------------------------------------

/// `enigo::Enigo` holds raw OS handles (CGEventSource, X11/Xtest, Win32 hooks)
/// that the compiler cannot prove are thread-safe on every platform, even
/// though it is only ever touched by one thread at a time here.
///
/// Safety: the value is exclusively accessed through the enclosing
/// `Mutex`, so concurrent use is impossible. Moving the whole injector between
/// async tasks is sound because the platform backends permit any single
/// thread to use the handle — just not two threads at once, which the mutex
/// already forbids. This mirrors `unsafe impl Send` that enigo itself applies
/// to its Linux connection types.
struct SendEnigo(enigo::Enigo);
unsafe impl Send for SendEnigo {}

/// Real input injection backend built on `enigo`.
pub struct EnigoInjector {
    inner: Mutex<SendEnigo>,
}

impl std::fmt::Debug for EnigoInjector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EnigoInjector")
    }
}

impl EnigoInjector {
    pub fn new() -> Result<Self> {
        let settings = enigo::Settings::default();
        let inner =
            enigo::Enigo::new(&settings).context("failed to init enigo input injector")?;
        Ok(Self {
            inner: Mutex::new(SendEnigo(inner)),
        })
    }
}

impl Default for EnigoInjector {
    fn default() -> Self {
        Self::new().expect("input injector must initialize")
    }
}

/// Map a forwarded key name (single char or a named key) to `enigo::Key`.
fn enigo_key(key: &str) -> Option<enigo::Key> {
    use enigo::Key;
    if key.chars().count() == 1 {
        return key.chars().next().map(Key::Unicode);
    }
    match key.to_lowercase().as_str() {
        "enter" | "return" => Some(Key::Return),
        "tab" => Some(Key::Tab),
        "space" | "spacebar" => Some(Key::Space),
        "backspace" | "delete" => Some(Key::Backspace),
        "escape" | "esc" => Some(Key::Escape),
        "up" | "arrowup" => Some(Key::UpArrow),
        "down" | "arrowdown" => Some(Key::DownArrow),
        "left" | "arrowleft" => Some(Key::LeftArrow),
        "right" | "arrowright" => Some(Key::RightArrow),
        "home" => Some(Key::Home),
        "end" => Some(Key::End),
        "pageup" => Some(Key::PageUp),
        "pagedown" => Some(Key::PageDown),
        "f1" => Some(Key::F1),
        "f2" => Some(Key::F2),
        "f3" => Some(Key::F3),
        "f4" => Some(Key::F4),
        "f5" => Some(Key::F5),
        "f6" => Some(Key::F6),
        "f7" => Some(Key::F7),
        "f8" => Some(Key::F8),
        "f9" => Some(Key::F9),
        "f10" => Some(Key::F10),
        "f11" => Some(Key::F11),
        "f12" => Some(Key::F12),
        "shift" => Some(Key::Shift),
        "ctrl" | "control" => Some(Key::Control),
        "alt" | "option" => Some(Key::Alt),
        "cmd" | "meta" | "super" | "win" | "windows" => Some(Key::Meta),
        "capslock" => Some(Key::CapsLock),
        _ => None,
    }
}

/// Map a forwarded modifier name to `enigo::Key`.
fn modifier_key(name: &str) -> Option<enigo::Key> {
    match name.to_lowercase().as_str() {
        "shift" => Some(enigo::Key::Shift),
        "ctrl" | "control" => Some(enigo::Key::Control),
        "alt" | "option" => Some(enigo::Key::Alt),
        "cmd" | "meta" | "super" | "win" | "windows" => Some(enigo::Key::Meta),
        _ => None,
    }
}

fn inject_keyboard(enigo: &mut SendEnigo, event: KeyEvent) -> Result<()> {
    use enigo::{Direction, Keyboard};
    let (key_name, modifiers, is_down) = match event {
        KeyEvent::KeyDown { key, modifiers } => (key, modifiers, true),
        KeyEvent::KeyUp { key, modifiers } => (key, modifiers, false),
    };
    let key = enigo_key(&key_name).ok_or_else(|| anyhow::anyhow!("unknown key: {key_name}"))?;

    let mut mod_keys = Vec::new();
    for m in &modifiers {
        if let Some(k) = modifier_key(m) {
            mod_keys.push(k);
        }
    }

    if is_down {
        for k in &mod_keys {
            enigo.0.key(*k, Direction::Press)?;
        }
        enigo.0.key(key, Direction::Press)?;
    } else {
        enigo.0.key(key, Direction::Release)?;
        for k in mod_keys.iter().rev() {
            enigo.0.key(*k, Direction::Release)?;
        }
    }
    Ok(())
}

fn inject_mouse(enigo: &mut SendEnigo, event: MouseEvent) -> Result<()> {
    use enigo::{Axis, Button, Coordinate, Direction, Mouse};
    match event {
        MouseEvent::Move { x, y } => {
            enigo.0.move_mouse(x as i32, y as i32, Coordinate::Abs)?;
        }
        MouseEvent::ButtonDown { button, x, y } => {
            enigo.0.move_mouse(x as i32, y as i32, Coordinate::Abs)?;
            let b = match button {
                1 => Button::Right,
                2 => Button::Middle,
                _ => Button::Left,
            };
            enigo.0.button(b, Direction::Press)?;
        }
        MouseEvent::ButtonUp { button, x, y } => {
            enigo.0.move_mouse(x as i32, y as i32, Coordinate::Abs)?;
            let b = match button {
                1 => Button::Right,
                2 => Button::Middle,
                _ => Button::Left,
            };
            enigo.0.button(b, Direction::Release)?;
        }
        MouseEvent::Scroll { delta_x, delta_y } => {
            if delta_y != 0.0 {
                enigo.0.scroll(delta_y as i32, Axis::Vertical)?;
            }
            if delta_x != 0.0 {
                enigo.0.scroll(delta_x as i32, Axis::Horizontal)?;
            }
        }
    }
    Ok(())
}

impl InputInjector for EnigoInjector {
    fn inject(&self, action: &InputAction) -> Result<()> {
        let mut enigo = self
            .inner
            .lock()
            .map_err(|_| anyhow::anyhow!("input mutex poisoned"))?;
        match action {
            InputAction::Keyboard(event) => inject_keyboard(&mut enigo, event.clone()),
            InputAction::Mouse(event) => inject_mouse(&mut enigo, event.clone()),
        }
    }
}

// ---------------------------------------------------------------------------
// Stub fallback (headless CI / unsupported environments)
// ---------------------------------------------------------------------------

/// No-op backend used when the real injector cannot be opened (headless CI).
#[derive(Debug, Clone)]
pub struct StubInputInjector;

impl InputInjector for StubInputInjector {
    fn available(&self) -> bool { false }
    fn inject(&self, action: &InputAction) -> Result<()> {
        tracing::debug!("[INPUT] Stub inject: {action:?}");
        Ok(())
    }
}

/// Factory: return the real enigo backend, falling back to the stub when the
/// platform input system is unavailable.
pub fn create_input_injector() -> Box<dyn InputInjector> {
    match EnigoInjector::new() {
        Ok(injector) => Box::new(injector),
        Err(e) => {
            tracing::warn!("[INPUT] Falling back to stub injector: {e}");
            Box::new(StubInputInjector)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_single_char_to_unicode() {
        assert_eq!(enigo_key("a"), Some(enigo::Key::Unicode('a')));
        assert_eq!(enigo_key("Z"), Some(enigo::Key::Unicode('Z')));
        assert_eq!(enigo_key("0"), Some(enigo::Key::Unicode('0')));
        assert_eq!(enigo_key(" "), Some(enigo::Key::Unicode(' ')));
    }

    #[test]
    fn maps_named_keys_case_insensitively() {
        assert_eq!(enigo_key("Enter"), Some(enigo::Key::Return));
        assert_eq!(enigo_key("enter"), Some(enigo::Key::Return));
        assert_eq!(enigo_key("TAB"), Some(enigo::Key::Tab));
        assert_eq!(enigo_key("ArrowUp"), Some(enigo::Key::UpArrow));
        assert_eq!(enigo_key("arrowdown"), Some(enigo::Key::DownArrow));
        assert_eq!(enigo_key("LEFT"), Some(enigo::Key::LeftArrow));
        assert_eq!(enigo_key("f5"), Some(enigo::Key::F5));
        assert_eq!(enigo_key("space"), Some(enigo::Key::Space));
        assert_eq!(enigo_key("backspace"), Some(enigo::Key::Backspace));
        assert_eq!(enigo_key("escape"), Some(enigo::Key::Escape));
    }

    #[test]
    fn maps_modifier_aliases() {
        assert_eq!(modifier_key("Shift"), Some(enigo::Key::Shift));
        assert_eq!(modifier_key("ctrl"), Some(enigo::Key::Control));
        assert_eq!(modifier_key("control"), Some(enigo::Key::Control));
        assert_eq!(modifier_key("alt"), Some(enigo::Key::Alt));
        assert_eq!(modifier_key("option"), Some(enigo::Key::Alt));
        assert_eq!(modifier_key("cmd"), Some(enigo::Key::Meta));
        assert_eq!(modifier_key("meta"), Some(enigo::Key::Meta));
        assert_eq!(modifier_key("win"), Some(enigo::Key::Meta));
    }

    #[test]
    fn rejects_unknown_keys() {
        assert_eq!(enigo_key("nonsense-key"), None);
        assert_eq!(modifier_key("hyper"), None);
    }

    #[test]
    fn stub_injector_is_a_noop() {
        let stub = StubInputInjector;
        let ok = stub.inject(&InputAction::Keyboard(KeyEvent::KeyDown {
            key: "a".into(),
            modifiers: vec![],
        }));
        assert!(ok.is_ok());
    }
}
