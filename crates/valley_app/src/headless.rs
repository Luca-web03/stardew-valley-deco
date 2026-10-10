use engine::{DrawList, Key, ModuleHost, MouseButton, Vec2};

use crate::Args;

pub fn parse_key(s: &str) -> Option<Key> {
    Some(match s.to_ascii_lowercase().as_str() {
        "w" => Key::W,
        "a" => Key::A,
        "s" => Key::S,
        "d" => Key::D,
        "e" => Key::E,
        "f" => Key::F,
        "i" => Key::I,
        "m" => Key::M,
        "j" => Key::J,
        "q" => Key::Q,
        "r" => Key::R,
        "t" => Key::T,
        "x" => Key::X,
        "c" => Key::C,
        "y" => Key::Y,
        "n" => Key::N,
        "p" => Key::P,
        "tab" => Key::Tab,
        "up" => Key::Up,
        "down" => Key::Down,
        "left" => Key::Left,
        "right" => Key::Right,
        "esc" | "escape" => Key::Escape,
        "enter" | "return" => Key::Enter,
        "space" => Key::Space,
        "backspace" => Key::Backspace,
        "shift" => Key::LeftShift,
        "ctrl" => Key::LeftCtrl,
        "1" => Key::Num1,
        "2" => Key::Num2,
        "3" => Key::Num3,
        "4" => Key::Num4,
        "5" => Key::Num5,
        "6" => Key::Num6,
        "7" => Key::Num7,
        "8" => Key::Num8,
        "9" => Key::Num9,
        "0" => Key::Num0,
        "-" | "minus" => Key::Minus,
        "=" | "equal" => Key::Equal,
        _ => return None,
    })
}

fn screenshot(host: &mut ModuleHost, path: &str) -> Result<(), String> {
    let mut list = DrawList::new(host.ctx.screen_w, host.ctx.screen_h);
    host.draw(&mut list);
    let img = engine::soft::render(&list, &host.ctx.textures);
    if let Some(dir) = std::path::Path::new(path).parent() {
        if !dir.as_os_str().is_empty() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
    }
    std::fs::write(path, img.encode_png()).map_err(|e| e.to_string())?;
    eprintln!("wrote {path}");
    Ok(())
}

pub fn run_script(host: &mut ModuleHost, script: &str) -> Result<(), String> {
    for step in script.split_whitespace() {
        let parts: Vec<&str> = step.split(':').collect();
        match parts.as_slice() {
            ["wait", n] => {
                for _ in 0..n.parse::<u32>().map_err(|e| e.to_string())? {
                    host.tick();
                }
            }
            ["hold", k, n] => {
                let key = parse_key(k).ok_or(format!("unknown key {k}"))?;
                host.ctx.input.key_down(key);
                for _ in 0..n.parse::<u32>().map_err(|e| e.to_string())? {
                    host.tick();
                }
                host.ctx.input.key_up(key);
                host.tick();
            }
            ["tap", k] => {
                let key = parse_key(k).ok_or(format!("unknown key {k}"))?;
                host.ctx.input.key_down(key);
                host.tick();
                host.ctx.input.key_up(key);
                host.tick();
            }
            [b @ ("click" | "rclick"), x, y] => {
                let btn = if *b == "click" { MouseButton::Left } else { MouseButton::Right };
                host.ctx.input.mouse = Vec2::new(x.parse().map_err(|_| "bad x")?, y.parse().map_err(|_| "bad y")?);
                host.ctx.input.mouse_down(btn);
                host.tick();
                host.ctx.input.mouse_up(btn);
                host.tick();
            }
            ["move", x, y] => {
                host.ctx.input.mouse = Vec2::new(x.parse().map_err(|_| "bad x")?, y.parse().map_err(|_| "bad y")?);
                host.tick();
            }
            ["type", text] => {
                host.ctx.input.text = text.replace('_', " ");
                host.tick();
            }
            ["shot", path] => screenshot(host, path)?,
            ["cmd", c] => {
                let r = host.command(&c.replace('_', " "));
                eprintln!("> {} : {r}", c.replace('_', " "));
            }
            _ => return Err(format!("bad script step {step}")),
        }
        // Headless runs never open an audio device; discard sound requests.
        host.ctx.audio.drain();
        if host.ctx.quit_requested {
            break;
        }
    }
    Ok(())
}

pub fn run(args: &Args) -> i32 {
    let mut host = ModuleHost::new(args.width, args.height);
    host.add(Box::new(crate::build_game(args)));
    host.tick();
    if let Err(e) = run_script(&mut host, &args.script) {
        eprintln!("script error: {e}");
        return 2;
    }
    if let Some(p) = &args.screenshot {
        if let Err(e) = screenshot(&mut host, &p.to_string_lossy()) {
            eprintln!("screenshot failed: {e}");
            return 1;
        }
    }
    0
}
