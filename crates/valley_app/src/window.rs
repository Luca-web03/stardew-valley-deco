use std::collections::HashMap;

use engine::{Color as EColor, DrawCmd, DrawList, Key, ModuleHost, MouseButton, TexId, Vec2};
use macroquad::prelude as mq;

use crate::audio::AudioSystem;
use crate::Args;

pub(crate) const KEYS: &[(mq::KeyCode, Key)] = &[
    (mq::KeyCode::W, Key::W), (mq::KeyCode::A, Key::A), (mq::KeyCode::S, Key::S), (mq::KeyCode::D, Key::D),
    (mq::KeyCode::E, Key::E), (mq::KeyCode::F, Key::F), (mq::KeyCode::I, Key::I), (mq::KeyCode::M, Key::M),
    (mq::KeyCode::J, Key::J), (mq::KeyCode::Q, Key::Q), (mq::KeyCode::R, Key::R), (mq::KeyCode::T, Key::T),
    (mq::KeyCode::X, Key::X), (mq::KeyCode::C, Key::C), (mq::KeyCode::Y, Key::Y), (mq::KeyCode::N, Key::N),
    (mq::KeyCode::P, Key::P), (mq::KeyCode::Tab, Key::Tab),
    (mq::KeyCode::Up, Key::Up), (mq::KeyCode::Down, Key::Down), (mq::KeyCode::Left, Key::Left), (mq::KeyCode::Right, Key::Right),
    (mq::KeyCode::Escape, Key::Escape), (mq::KeyCode::Enter, Key::Enter), (mq::KeyCode::KpEnter, Key::Enter),
    (mq::KeyCode::Space, Key::Space), (mq::KeyCode::Backspace, Key::Backspace), (mq::KeyCode::Delete, Key::Delete),
    (mq::KeyCode::LeftShift, Key::LeftShift), (mq::KeyCode::RightShift, Key::RightShift), (mq::KeyCode::LeftControl, Key::LeftCtrl),
    (mq::KeyCode::Key0, Key::Num0), (mq::KeyCode::Key1, Key::Num1), (mq::KeyCode::Key2, Key::Num2), (mq::KeyCode::Key3, Key::Num3),
    (mq::KeyCode::Key4, Key::Num4), (mq::KeyCode::Key5, Key::Num5), (mq::KeyCode::Key6, Key::Num6), (mq::KeyCode::Key7, Key::Num7),
    (mq::KeyCode::Key8, Key::Num8), (mq::KeyCode::Key9, Key::Num9), (mq::KeyCode::Minus, Key::Minus), (mq::KeyCode::Equal, Key::Equal),
    (mq::KeyCode::F1, Key::F1), (mq::KeyCode::F2, Key::F2), (mq::KeyCode::F3, Key::F3), (mq::KeyCode::F4, Key::F4),
    (mq::KeyCode::F5, Key::F5), (mq::KeyCode::F9, Key::F9), (mq::KeyCode::F10, Key::F10), (mq::KeyCode::F11, Key::F11), (mq::KeyCode::F12, Key::F12),
];

pub(crate) fn col(c: EColor) -> mq::Color {
    mq::Color::from_rgba(c.r, c.g, c.b, c.a)
}

/// GPU copies of engine textures, refreshed when an image is replaced.
#[derive(Default)]
pub(crate) struct GpuTextures {
    map: HashMap<TexId, (u32, mq::Texture2D)>,
}

impl GpuTextures {
    fn get(&mut self, store: &engine::TextureStore, id: TexId) -> &mq::Texture2D {
        let gen = store.generation(id);
        let stale = self.map.get(&id).map(|(g, _)| *g != gen).unwrap_or(true);
        if stale {
            let img = store.get(id);
            let t = mq::Texture2D::from_rgba8(img.width as u16, img.height as u16, &img.rgba);
            t.set_filter(mq::FilterMode::Nearest);
            self.map.insert(id, (gen, t));
        }
        &self.map[&id].1
    }
}

pub(crate) fn present(list: &DrawList, store: &engine::TextureStore, gpu: &mut GpuTextures, scale: f32, offset: mq::Vec2) {
    mq::clear_background(col(list.clear));
    for cmd in list.sorted() {
        match cmd {
            DrawCmd::Fill { rect, color, .. } => {
                mq::draw_rectangle(offset.x + rect.x * scale, offset.y + rect.y * scale, rect.w * scale, rect.h * scale, col(*color));
            }
            DrawCmd::Sprite(s) => {
                let tex = gpu.get(store, s.tex).clone();
                let x = offset.x + s.dst.x * scale;
                let y = offset.y + s.dst.y * scale;
                let params = mq::DrawTextureParams {
                    dest_size: Some(mq::vec2(s.dst.w * scale, s.dst.h * scale)),
                    source: Some(mq::Rect::new(s.src.x as f32, s.src.y as f32, s.src.w as f32, s.src.h as f32)),
                    rotation: s.rotation,
                    flip_x: s.flip_x,
                    flip_y: s.flip_y,
                    pivot: Some(mq::vec2(x + s.origin.x * scale, y + s.origin.y * scale)),
                };
                mq::draw_texture_ex(&tex, x, y, col(s.tint), params);
            }
        }
    }
}

pub fn run(args: Args) {
    let conf = mq::Conf {
        window_title: "valley-rs".to_string(),
        window_width: args.width as i32,
        window_height: args.height as i32,
        high_dpi: false,
        window_resizable: true,
        ..Default::default()
    };
    macroquad::Window::from_config(conf, async move {
        let mut host = ModuleHost::new(args.width, args.height);
        let game = crate::build_game(&args);
        let library = valley_content::audio::AudioLibrary::open(&game.cm);
        let mut audio = if args.mute { AudioSystem::silent(library) } else { AudioSystem::start(library) };
        host.add(Box::new(game));
        let mut gpu = GpuTextures::default();
        let mut list = DrawList::new(args.width, args.height);
        let mut pads = crate::gamepad::Gamepads::new();
        let mut last_os_mouse = (f32::NAN, f32::NAN);
        loop {
            // Logical resolution follows the window (UI lays out to it).
            let (ww, wh) = (mq::screen_width(), mq::screen_height());
            host.ctx.screen_w = ww.max(320.0) as u32;
            host.ctx.screen_h = wh.max(240.0) as u32;

            for (mk, k) in KEYS {
                if mq::is_key_pressed(*mk) {
                    host.ctx.input.key_down(*k);
                }
                if mq::is_key_released(*mk) {
                    host.ctx.input.key_up(*k);
                }
            }
            while let Some(c) = mq::get_char_pressed() {
                if !c.is_control() {
                    host.ctx.input.text.push(c);
                }
            }
            // Take the OS cursor only when it moves, so a gamepad-driven
            // cursor isn't snapped back every frame.
            let (mx, my) = mq::mouse_position();
            if (mx, my) != last_os_mouse {
                host.ctx.input.mouse = Vec2::new(mx, my);
                last_os_mouse = (mx, my);
            }
            for (mb, b) in [(mq::MouseButton::Left, MouseButton::Left), (mq::MouseButton::Right, MouseButton::Right), (mq::MouseButton::Middle, MouseButton::Middle)] {
                if mq::is_mouse_button_pressed(mb) {
                    host.ctx.input.mouse_down(b);
                }
                if mq::is_mouse_button_released(mb) {
                    host.ctx.input.mouse_up(b);
                }
            }
            let (_, wheel) = mq::mouse_wheel();
            if wheel != 0.0 {
                host.ctx.input.wheel = wheel.signum();
            }
            pads.poll(&mut host.ctx.input, (host.ctx.screen_w as f32, host.ctx.screen_h as f32));

            // Run at least one tick per frame so single-frame presses register.
            if host.advance(mq::get_frame_time() as f64) == 0 {
                // Keep edge-triggered input until a tick consumes it.
            }
            audio.process(host.ctx.audio.drain());
            if host.ctx.quit_requested {
                break;
            }
            host.draw(&mut list);
            present(&list, &host.ctx.textures, &mut gpu, 1.0, mq::vec2(0.0, 0.0));
            mq::next_frame().await;
        }
    });
}
