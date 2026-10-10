//! Default key bindings (the classic PC layout) mapped to player intents.

use engine::{InputState, Key, MouseButton};
use valley_sim::input::PlayerInput;
use valley_sim::TilePos;

pub const SLOT_KEYS: [Key; 12] = [
    Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5, Key::Num6,
    Key::Num7, Key::Num8, Key::Num9, Key::Num0, Key::Minus, Key::Equal,
];

/// Rebindable actions (two keys each; mouse buttons are fixed).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Bindings {
    pub up: Vec<Key>,
    pub down: Vec<Key>,
    pub left: Vec<Key>,
    pub right: Vec<Key>,
    pub use_tool: Vec<Key>,
    pub action: Vec<Key>,
    pub menu: Vec<Key>,
    pub journal: Vec<Key>,
    pub walk: Vec<Key>,
}

impl Default for Bindings {
    fn default() -> Self {
        Self {
            up: vec![Key::W, Key::Up],
            down: vec![Key::S, Key::Down],
            left: vec![Key::A, Key::Left],
            right: vec![Key::D, Key::Right],
            use_tool: vec![Key::C],
            action: vec![Key::X],
            menu: vec![Key::E, Key::I],
            journal: vec![Key::F],
            walk: vec![Key::LeftShift, Key::RightShift],
        }
    }
}

pub const BINDING_NAMES: [&str; 9] = ["Move up", "Move down", "Move left", "Move right", "Use tool", "Check / act", "Menu", "Journal", "Walk"];

impl Bindings {
    pub fn get_mut(&mut self, i: usize) -> &mut Vec<Key> {
        match i {
            0 => &mut self.up,
            1 => &mut self.down,
            2 => &mut self.left,
            3 => &mut self.right,
            4 => &mut self.use_tool,
            5 => &mut self.action,
            6 => &mut self.menu,
            7 => &mut self.journal,
            _ => &mut self.walk,
        }
    }
    pub fn get(&self, i: usize) -> &Vec<Key> {
        match i {
            0 => &self.up,
            1 => &self.down,
            2 => &self.left,
            3 => &self.right,
            4 => &self.use_tool,
            5 => &self.action,
            6 => &self.menu,
            7 => &self.journal,
            _ => &self.walk,
        }
    }
}

thread_local! {
    static BINDINGS: std::cell::RefCell<Bindings> = std::cell::RefCell::new(Bindings::default());
}

/// Install the active bindings (from the options).
pub fn set_bindings(b: &Bindings) {
    BINDINGS.with(|x| *x.borrow_mut() = b.clone());
}

fn held_any(i: &InputState, keys: &[Key]) -> bool {
    keys.iter().any(|k| i.held(*k))
}

fn pressed_any(i: &InputState, keys: &[Key]) -> bool {
    keys.iter().any(|k| i.pressed(*k))
}

pub fn menu_pressed(i: &InputState) -> bool {
    i.pressed(Key::Escape) || BINDINGS.with(|b| pressed_any(i, &b.borrow().menu))
}

pub fn confirm_pressed(i: &InputState) -> bool {
    i.pressed(Key::Enter)
        || i.pressed(Key::Space)
        || BINDINGS.with(|b| pressed_any(i, &b.borrow().action) || pressed_any(i, &b.borrow().use_tool))
        || i.mouse_pressed(MouseButton::Left)
        || i.mouse_pressed(MouseButton::Right)
}

pub fn player_input(i: &InputState, cursor_tile: Option<TilePos>) -> PlayerInput {
    let b = BINDINGS.with(|b| b.borrow().clone());
    let mut p = PlayerInput::default();
    if held_any(i, &b.left) {
        p.move_x -= 1;
    }
    if held_any(i, &b.right) {
        p.move_x += 1;
    }
    if held_any(i, &b.up) {
        p.move_y -= 1;
    }
    if held_any(i, &b.down) {
        p.move_y += 1;
    }
    p.use_tool = pressed_any(i, &b.use_tool) || i.mouse_pressed(MouseButton::Left);
    p.use_held = held_any(i, &b.use_tool) || i.mouse_held(MouseButton::Left);
    p.action = pressed_any(i, &b.action) || i.mouse_pressed(MouseButton::Right);
    // Mouse targets the clicked tile; keyboard uses the faced tile.
    // Holding the mouse button keeps tracking it (slingshot aiming).
    if i.mouse_pressed(MouseButton::Left) || i.mouse_pressed(MouseButton::Right) || i.mouse_held(MouseButton::Left) {
        p.cursor_tile = cursor_tile;
    }
    p.walk = held_any(i, &b.walk);
    for (n, k) in SLOT_KEYS.iter().enumerate() {
        if i.pressed(*k) {
            p.select_slot = Some(n);
        }
    }
    if i.wheel > 0.0 {
        p.scroll = -1;
    } else if i.wheel < 0.0 {
        p.scroll = 1;
    }
    p
}

pub fn journal_pressed(i: &InputState) -> bool {
    BINDINGS.with(|b| pressed_any(i, &b.borrow().journal))
}
