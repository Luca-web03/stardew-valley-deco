//! Default key bindings (the classic PC layout) mapped to player intents.

use engine::{InputState, Key, MouseButton};
use valley_sim::input::PlayerInput;
use valley_sim::TilePos;

pub const SLOT_KEYS: [Key; 12] = [
    Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5, Key::Num6,
    Key::Num7, Key::Num8, Key::Num9, Key::Num0, Key::Minus, Key::Equal,
];

pub fn menu_pressed(i: &InputState) -> bool {
    i.pressed(Key::E) || i.pressed(Key::Escape) || i.pressed(Key::I)
}

pub fn confirm_pressed(i: &InputState) -> bool {
    i.pressed(Key::Enter) || i.pressed(Key::Space) || i.pressed(Key::X) || i.pressed(Key::C) || i.mouse_pressed(MouseButton::Left) || i.mouse_pressed(MouseButton::Right)
}

pub fn player_input(i: &InputState, cursor_tile: Option<TilePos>) -> PlayerInput {
    let mut p = PlayerInput::default();
    if i.held(Key::A) || i.held(Key::Left) {
        p.move_x -= 1;
    }
    if i.held(Key::D) || i.held(Key::Right) {
        p.move_x += 1;
    }
    if i.held(Key::W) || i.held(Key::Up) {
        p.move_y -= 1;
    }
    if i.held(Key::S) || i.held(Key::Down) {
        p.move_y += 1;
    }
    p.use_tool = i.pressed(Key::C) || i.mouse_pressed(MouseButton::Left);
    p.use_held = i.held(Key::C) || i.mouse_held(MouseButton::Left);
    p.action = i.pressed(Key::X) || i.mouse_pressed(MouseButton::Right);
    // Mouse targets the clicked tile; keyboard uses the faced tile.
    if i.mouse_pressed(MouseButton::Left) || i.mouse_pressed(MouseButton::Right) {
        p.cursor_tile = cursor_tile;
    }
    p.walk = i.held(Key::LeftShift) || i.held(Key::RightShift);
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
