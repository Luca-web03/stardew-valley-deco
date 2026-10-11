//! Co-op client: send input to the host, show the frames it streams back.
//! Needs no game content (textures arrive over the network).

use engine::{Key, TextureStore};
use valley_game::net::{mouse_code, InputEvent, NetClient};

use crate::Args;

fn connect(args: &Args) -> Option<NetClient> {
    let addr = args.join.clone()?;
    let addr = if addr.contains(':') { addr } else { format!("{addr}:24642") };
    let name = args.name.clone().unwrap_or_else(|| "Farmhand".into());
    match NetClient::join(addr.as_str(), &name) {
        Ok(c) => {
            eprintln!("connected to {addr} as {name}");
            Some(c)
        }
        Err(e) => {
            eprintln!("could not connect to {addr}: {e}");
            None
        }
    }
}

pub fn run(args: Args) -> i32 {
    if args.headless {
        return run_headless(&args);
    }
    run_window(args);
    0
}

/// Scripted client (tests and demos): same step syntax as the headless
/// game runner; `shot:` saves the latest streamed frame.
fn run_headless(args: &Args) -> i32 {
    let Some(mut client) = connect(args) else { return 1 };
    let mut store = TextureStore::new();
    client.push_input(InputEvent::Screen(args.width, args.height));
    let tick = |client: &mut NetClient, store: &mut TextureStore| {
        client.pump(store);
        std::thread::sleep(std::time::Duration::from_millis(16));
    };
    for step in args.script.split_whitespace() {
        let parts: Vec<&str> = step.split(':').collect();
        match parts.as_slice() {
            ["wait", n] => {
                for _ in 0..n.parse::<u32>().unwrap_or(1) {
                    tick(&mut client, &mut store);
                }
            }
            ["hold", k, n] => {
                let Some(key) = crate::headless::parse_key(k) else { continue };
                client.push_input(InputEvent::KeyDown(key));
                for _ in 0..n.parse::<u32>().unwrap_or(1) {
                    tick(&mut client, &mut store);
                }
                client.push_input(InputEvent::KeyUp(key));
                tick(&mut client, &mut store);
            }
            ["tap", k] => {
                let Some(key) = crate::headless::parse_key(k) else { continue };
                client.push_input(InputEvent::KeyDown(key));
                tick(&mut client, &mut store);
                client.push_input(InputEvent::KeyUp(key));
                tick(&mut client, &mut store);
            }
            ["click", x, y] => {
                let (x, y) = (x.parse().unwrap_or(0.0), y.parse().unwrap_or(0.0));
                client.push_input(InputEvent::MouseMove(x, y));
                client.push_input(InputEvent::MouseDown(0));
                tick(&mut client, &mut store);
                client.push_input(InputEvent::MouseUp(0));
                tick(&mut client, &mut store);
            }
            ["shot", path] => {
                // Let the newest frame arrive first.
                for _ in 0..10 {
                    tick(&mut client, &mut store);
                }
                let Some(frame) = &client.frame else {
                    eprintln!("no frame received yet");
                    return 1;
                };
                let img = engine::soft::render(frame, &store);
                if let Err(e) = std::fs::write(path, img.encode_png()) {
                    eprintln!("could not write {path}: {e}");
                    return 1;
                }
                eprintln!("wrote {path}");
            }
            _ => eprintln!("ignoring step {step}"),
        }
        if client.conn.closed {
            eprintln!("disconnected");
            return 1;
        }
    }
    0
}

fn run_window(args: Args) {
    use macroquad::prelude as mq;
    let conf = mq::Conf {
        window_title: "valley-rs (co-op)".to_string(),
        window_width: args.width as i32,
        window_height: args.height as i32,
        high_dpi: false,
        window_resizable: true,
        ..Default::default()
    };
    macroquad::Window::from_config(conf, async move {
        let Some(mut client) = connect(&args) else { return };
        let mut store = TextureStore::new();
        let mut gpu = crate::window::GpuTextures::default();
        let mut audio = if args.mute {
            crate::audio::AudioSystem::silent(valley_content::audio::AudioLibrary::open(&valley_content::ContentManager::with_game_dir(args.content.as_deref())))
        } else {
            crate::audio::AudioSystem::start(valley_content::audio::AudioLibrary::open(&valley_content::ContentManager::with_game_dir(args.content.as_deref())))
        };
        let mut last_size = (0u32, 0u32);
        let mut last_mouse = (f32::NAN, f32::NAN);
        loop {
            let size = (mq::screen_width() as u32, mq::screen_height() as u32);
            if size != last_size {
                client.push_input(InputEvent::Screen(size.0, size.1));
                last_size = size;
            }
            for (mk, k) in crate::window::KEYS {
                if mq::is_key_pressed(*mk) {
                    client.push_input(InputEvent::KeyDown(*k));
                }
                if mq::is_key_released(*mk) {
                    client.push_input(InputEvent::KeyUp(*k));
                }
            }
            let mut text = String::new();
            while let Some(c) = mq::get_char_pressed() {
                if !c.is_control() {
                    text.push(c);
                }
            }
            if !text.is_empty() {
                client.push_input(InputEvent::Text(text));
            }
            let (mx, my) = mq::mouse_position();
            if (mx, my) != last_mouse {
                client.push_input(InputEvent::MouseMove(mx, my));
                last_mouse = (mx, my);
            }
            for (mb, b) in [(mq::MouseButton::Left, engine::MouseButton::Left), (mq::MouseButton::Right, engine::MouseButton::Right), (mq::MouseButton::Middle, engine::MouseButton::Middle)] {
                if mq::is_mouse_button_pressed(mb) {
                    client.push_input(InputEvent::MouseDown(mouse_code(b)));
                }
                if mq::is_mouse_button_released(mb) {
                    client.push_input(InputEvent::MouseUp(mouse_code(b)));
                }
            }
            let (_, wheel) = mq::mouse_wheel();
            if wheel != 0.0 {
                client.push_input(InputEvent::Wheel(wheel.signum()));
            }
            client.pump(&mut store);
            audio.process(std::mem::take(&mut client.sounds));
            if client.conn.closed || mq::is_key_pressed(mq::KeyCode::F12) && mq::is_key_down(mq::KeyCode::LeftControl) {
                break;
            }
            match &client.frame {
                Some(f) => crate::window::present(f, &store, &mut gpu, 1.0, mq::vec2(0.0, 0.0)),
                None => {
                    mq::clear_background(mq::Color::from_rgba(20, 20, 30, 255));
                    mq::draw_text("Connecting...", 40.0, 60.0, 32.0, mq::WHITE);
                }
            }
            let _ = Key::Escape;
            mq::next_frame().await;
        }
    });
}
