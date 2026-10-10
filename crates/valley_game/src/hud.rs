//! Always-on HUD: clock/date/money panel, energy bar, toolbar, toasts.

use engine::{Color, FRect, Vec2};
use valley_render::ui::{self, Ui};
use valley_render::world_draw::{item_sprite, RenderCtx};
use valley_sim::inventory::TOOLBAR;
use valley_sim::item::Item;
use valley_sim::time::Weather;
use valley_sim::world::World;

pub const SLOT: f32 = 64.0;
pub const SLOT_GAP: f32 = 4.0;

/// Screen rect of a toolbar slot.
pub fn toolbar_slot_rect(screen_w: f32, screen_h: f32, i: usize) -> FRect {
    let total = TOOLBAR as f32 * (SLOT + SLOT_GAP) - SLOT_GAP;
    let x0 = (screen_w - total) / 2.0;
    FRect::new(x0 + i as f32 * (SLOT + SLOT_GAP), screen_h - SLOT - 24.0, SLOT, SLOT)
}

pub fn draw_item_in_slot(rc: &mut RenderCtx, ui: &mut Ui, w: &World, item: &Item, r: FRect) {
    let (tex, src) = item_sprite(rc, w, item);
    ui.icon(tex, src, FRect::new(r.x + 0.0, r.y + 0.0, r.w, r.h));
    ui.stack_number(item.stack(), r);
    if let Item::Object { quality, .. } = item {
        let star = match quality {
            valley_sim::item::Quality::Silver => Some(Color::rgb(200, 210, 230)),
            valley_sim::item::Quality::Gold => Some(Color::rgb(250, 210, 40)),
            valley_sim::item::Quality::Iridium => Some(Color::rgb(190, 90, 240)),
            _ => None,
        };
        if let Some(c) = star {
            ui.out.fill(FRect::new(r.x + 6.0, r.y + r.h - 16.0, 10.0, 10.0), c, ui.depth + 0.0008);
        }
    }
    if let Item::Tool(t) = item {
        if t.kind == valley_sim::item::ToolKind::WateringCan {
            let frac = t.water as f32 / t.water_capacity() as f32;
            ui.out.fill(FRect::new(r.x + 6.0, r.y + r.h - 10.0, r.w - 12.0, 4.0), Color::rgb(60, 40, 30), ui.depth + 0.0008);
            ui.out.fill(FRect::new(r.x + 6.0, r.y + r.h - 10.0, (r.w - 12.0) * frac, 4.0), Color::rgb(90, 160, 240), ui.depth + 0.0009);
        }
    }
}

pub fn draw_hud(rc: &mut RenderCtx, ui: &mut Ui, w: &World, sw: f32, sh: f32, toasts: &[(String, u32)]) {
    // Date / time / money panel.
    let r = FRect::new(sw - 300.0, 12.0, 288.0, 150.0);
    ui.panel(r);
    let date = format!("{}. {}", w.clock.weekday(), w.clock.day);
    ui.text(&date, Vec2::new(r.x + 24.0, r.y + 20.0), 3.0, ui::TEXT);
    let weather = match w.weather {
        Weather::Sun => "Sun",
        Weather::Rain => "Rain",
        Weather::Storm => "Storm",
        Weather::Snow => "Snow",
        Weather::Wind => "Wind",
    };
    ui.text(&format!("{} {}", w.clock.season.name(), weather), Vec2::new(r.x + 24.0, r.y + 52.0), 2.0, ui::TEXT);
    ui.text(&w.time_string(), Vec2::new(r.x + 24.0, r.y + 76.0), 3.0, ui::TEXT);
    let money = format!("{:>8}g", w.farmer.money);
    ui.out.fill(FRect::new(r.x + 16.0, r.y + 108.0, r.w - 32.0, 28.0), Color::rgb(90, 50, 30), ui.depth + 0.0001);
    ui.text(&money, Vec2::new(r.x + 28.0, r.y + 114.0), 2.0, Color::rgb(250, 220, 90));

    // Energy bar.
    let max = w.farmer.max_stamina as f32;
    let frac = (w.farmer.stamina / max).clamp(0.0, 1.0);
    let bar_h = 220.0;
    let er = FRect::new(sw - 52.0, sh - bar_h - 24.0, 36.0, bar_h);
    let col = Color::rgb(230, 60, 40).lerp(Color::rgb(80, 210, 60), frac);
    ui.vbar(er, frac, col);
    ui.text_shadow("E", Vec2::new(er.x + 10.0, er.y - 24.0), 2.0, ui::TEXT_LIGHT);
    if w.farmer.health < w.farmer.max_health {
        let hr = FRect::new(sw - 96.0, sh - bar_h - 24.0, 36.0, bar_h);
        ui.vbar(hr, w.farmer.health as f32 / w.farmer.max_health as f32, Color::rgb(220, 40, 40));
        ui.text_shadow("H", Vec2::new(hr.x + 10.0, hr.y - 24.0), 2.0, ui::TEXT_LIGHT);
    }

    // Toolbar.
    let row = w.farmer.current_slot / TOOLBAR;
    let first = toolbar_slot_rect(sw, sh, 0);
    let last = toolbar_slot_rect(sw, sh, TOOLBAR - 1);
    ui.panel(FRect::new(first.x - 16.0, first.y - 16.0, last.x + last.w - first.x + 32.0, first.h + 32.0));
    for i in 0..TOOLBAR {
        let slot = row * TOOLBAR + i;
        let r = toolbar_slot_rect(sw, sh, i);
        ui.slot(r, slot == w.farmer.current_slot);
        if let Some(item) = w.farmer.inventory.get(slot) {
            draw_item_in_slot(rc, ui, w, item, r);
        }
    }
    if let Some(item) = w.farmer.current_item() {
        let name = w.items.display_name(item);
        let m = ui.measure(&name, 2.0);
        ui.text_shadow(&name, Vec2::new((sw - m.x) / 2.0, first.y - 44.0), 2.0, ui::TEXT_LIGHT);
    }

    // Toasts (newest at the bottom).
    let mut y = sh - 140.0;
    for (msg, ms) in toasts.iter().rev().take(5) {
        let alpha = (*ms as f32 / 500.0).min(1.0);
        let m = ui.measure(msg, 2.0);
        let rr = FRect::new(16.0, y, m.x + 32.0, 40.0);
        ui.out.fill(rr, Color::rgba(70, 36, 20, (200.0 * alpha) as u8), ui.depth);
        ui.out.fill(FRect::new(rr.x + 3.0, rr.y + 3.0, rr.w - 6.0, rr.h - 6.0), Color::rgba(244, 214, 160, (235.0 * alpha) as u8), ui.depth + 0.00001);
        ui.text(msg, Vec2::new(rr.x + 16.0, rr.y + 12.0), 2.0, ui::TEXT.with_alpha(alpha));
        y -= 48.0;
    }
}

/// Fishing minigame and catch overlays (screen space).
pub fn draw_fishing_overlay(rc: &mut RenderCtx, ui: &mut Ui, w: &World, sw: f32, sh: f32) {
    use valley_sim::fishing::{FishingState, TRACK};
    match &w.fishing {
        FishingState::Minigame { game, .. } => {
            let scale = (sh - 140.0) / (TRACK + 40.0);
            let frame = FRect::new(sw / 2.0 + 120.0, 40.0, 120.0, (TRACK + 40.0) * scale);
            ui.panel(frame);
            let water = FRect::new(frame.x + 20.0, frame.y + 20.0 * scale, 48.0, TRACK * scale);
            ui.out.fill(water, Color::rgb(40, 90, 160), ui.depth + 0.001);
            let bar = FRect::new(water.x, water.y + game.bar_pos * scale, water.w, game.bar_height * scale);
            let c = if game.in_bar() { Color::rgba(110, 230, 90, 220) } else { Color::rgba(90, 190, 70, 170) };
            ui.out.fill(bar, c, ui.depth + 0.002);
            let fish_item = Item::object(&game.fish.id, 1);
            let (tex, src) = item_sprite(rc, w, &fish_item);
            ui.out.sprite(engine::Sprite::new(tex, src, FRect::new(water.x + 4.0, water.y + game.fish_pos * scale, 40.0, 40.0), ui.depth + 0.003));
            let prog = FRect::new(water.x + water.w + 12.0, water.y, 20.0, water.h);
            ui.out.fill(prog, Color::rgb(60, 40, 30), ui.depth + 0.001);
            let h = prog.h * game.progress.clamp(0.0, 1.0);
            let pc = Color::rgb(230, 70, 40).lerp(Color::rgb(80, 220, 60), game.progress);
            ui.out.fill(FRect::new(prog.x + 3.0, prog.y + prog.h - h, prog.w - 6.0, h), pc, ui.depth + 0.002);
        }
        FishingState::Caught { item, size, perfect, .. } => {
            let r = FRect::new(sw / 2.0 - 220.0, sh / 2.0 - 220.0, 440.0, 200.0);
            ui.panel(r);
            let (tex, src) = item_sprite(rc, w, item);
            ui.icon(tex, src, FRect::new(r.x + 32.0, r.y + 48.0, 96.0, 96.0));
            ui.text(&w.items.display_name(item), Vec2::new(r.x + 150.0, r.y + 48.0), 3.0, ui::TEXT);
            if *size > 0 {
                ui.text(&format!("Length: {size} in."), Vec2::new(r.x + 150.0, r.y + 92.0), 2.0, ui::TEXT);
            }
            if *perfect {
                ui.text("PERFECT!", Vec2::new(r.x + 150.0, r.y + 124.0), 3.0, Color::rgb(230, 150, 20));
            }
        }
        _ => {}
    }
}
