use serde::{Deserialize, Serialize};

/// Types of keyboard events that can be forwarded between devices.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyEvent {
    KeyDown { key: String, modifiers: Vec<String> },
    KeyUp { key: String, modifiers: Vec<String> },
}

/// Types of mouse events that can be forwarded between devices.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MouseEvent {
    Move { x: f64, y: f64 },
    ButtonDown { button: u8, x: f64, y: f64 },
    ButtonUp { button: u8, x: f64, y: f64 },
    Scroll { delta_x: f64, delta_y: f64 },
}

/// A single input action, wrapping either a keyboard or mouse event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputAction {
    Keyboard(KeyEvent),
    Mouse(MouseEvent),
}
