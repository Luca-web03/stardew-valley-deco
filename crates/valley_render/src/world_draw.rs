use engine::{Color, DrawList, FRect, IRect, Sprite, TextureStore, Vec2};
use valley_content::ContentManager;
use valley_sim::item::{Item, ToolKind};
use valley_sim::location::{seasonal_image, GameLocation};
use valley_sim::objects::ObjKind;
use valley_sim::terrain::TerrainFeature;
use valley_sim::world::World;
use valley_sim::{Dir, TilePos, TILE, TILE_F, ZOOM};

use crate::assets::Assets;
use crate::camera::Camera;
use crate::*;

/// Frame layout for 16x32 character sheets: rows down, right, up, left;
/// four walk frames per row.
pub fn character_src(facing: Dir, frame: i32) -> IRect {
    character_src_with(&crate::layouts::CharacterLayout::default(), facing, frame)
}

pub fn character_src_with(l: &crate::layouts::CharacterLayout, facing: Dir, frame: i32) -> IRect {
    let row = l.rows[match facing {
        Dir::Down => 0,
        Dir::Right => 1,
        Dir::Up => 2,
        Dir::Left => 3,
    }];
    IRect::new((frame % l.walk_frames.max(1)) * l.frame_w, row * l.frame_h, l.frame_w, l.frame_h)
}

fn walk_frame(moving: bool, walk_ms: u32, frame_ms: u32) -> i32 {
    if moving {
        ((walk_ms / frame_ms) % 4) as i32
    } else {
        0
    }
}

pub struct RenderCtx<'a> {
    pub cm: &'a mut ContentManager,
    pub textures: &'a mut TextureStore,
    pub assets: &'a mut Assets,
}

fn sort_depth(world_y: f32, map_h_px: f32) -> f32 {
    let t = (world_y / map_h_px.max(1.0)).clamp(0.0, 1.0);
    DEPTH_SORT_MIN + t * (DEPTH_SORT_MAX - DEPTH_SORT_MIN - 0.001)
}

/// Draw one map layer.
fn draw_layer(rc: &mut RenderCtx, out: &mut DrawList, cam: &Camera, loc: &GameLocation, layer: &str, depth: f32, season: &str, ms: u64) {
    let Some(l) = loc.map.layer(layer) else { return };
    if !l.visible {
        return;
    }
    let sheet_tex: Vec<_> = loc
        .map
        .sheets
        .iter()
        .map(|s| {
            let img = if loc.outdoors { seasonal_image(&s.image, season) } else { s.image.clone() };
            rc.assets.map_sheet(rc.cm, rc.textures, &img)
        })
        .collect();
    let (x0, y0, x1, y1) = cam.tile_range(TILE_F, 1);
    for y in y0.max(0)..=y1.min(l.height - 1) {
        for x in x0.max(0)..=x1.min(l.width - 1) {
            let Some(t) = l.get(x, y) else { continue };
            let (si, idx) = t.frame_at(ms);
            let Some(sheet) = loc.map.sheets.get(si as usize) else { continue };
            let (sx, sy, sw, sh) = sheet.src(idx);
            let p = cam.to_screen(Vec2::new((x * TILE) as f32, (y * TILE) as f32));
            // Slight per-row offset keeps rows ordered within a layer.
            out.draw(sheet_tex[si as usize], IRect::new(sx, sy, sw, sh), p, ZOOM, depth + y as f32 * 1e-6);
        }
    }
}

pub fn draw_world(rc: &mut RenderCtx, out: &mut DrawList, w: &World, cam: &Camera) {
    draw_location_view(rc, out, w, &w.current, cam);
}

/// Draw any location (the farmer only appears in the current one).
pub fn draw_location_view(rc: &mut RenderCtx, out: &mut DrawList, w: &World, loc_name: &str, cam: &Camera) {
    let Some(loc) = w.locations.get(loc_name) else { return };
    let is_current = loc_name == w.current;
    let season = w.clock.season.key();
    let ms = w.game_ms;
    let map_h = (loc.height() * TILE) as f32;

    draw_layer(rc, out, cam, loc, "Back", DEPTH_BACK, season, ms);
    draw_layer(rc, out, cam, loc, "Buildings", DEPTH_BUILDINGS, season, ms);

    let lay = rc.assets.layouts(rc.cm);
    // Tilled soil.
    let dirt_tex = rc.assets.tex(rc.cm, rc.textures, &lay.hoe_dirt.texture);
    let crops_tex = rc.assets.tex(rc.cm, rc.textures, "TileSheets/crops");
    for (p, t) in &loc.state.terrain {
        let TerrainFeature::HoeDirt(d) = t else { continue };
        let is_dirt = |q: TilePos| matches!(loc.terrain(q), Some(TerrainFeature::HoeDirt(_)));
        let mut mask = 0;
        if is_dirt(p.offset(Dir::Up)) {
            mask |= 1;
        }
        if is_dirt(p.offset(Dir::Right)) {
            mask |= 2;
        }
        if is_dirt(p.offset(Dir::Down)) {
            mask |= 4;
        }
        if is_dirt(p.offset(Dir::Left)) {
            mask |= 8;
        }
        let pos = cam.to_screen(Vec2::new((p.x * TILE) as f32, (p.y * TILE) as f32));
        let cols = lay.hoe_dirt.cols.max(1);
        let src = IRect::new((mask % cols) * 16, (mask / cols) * 16, 16, 16);
        out.draw(dirt_tex, src, pos, ZOOM, DEPTH_GROUND);
        if d.watered {
            let [ox, oy] = lay.hoe_dirt.watered_offset;
            out.draw(dirt_tex, IRect::new(ox + src.x, oy + src.y, 16, 16), pos, ZOOM, DEPTH_GROUND + 0.001);
        }
        if let Some(c) = &d.crop {
            let Some(data) = w.items.crops.get(&c.seed_id) else { continue };
            let frame = c.sprite_frame(data);
            let tex = if data.texture.is_empty() { crops_tex } else { rc.assets.tex(rc.cm, rc.textures, &data.texture) };
            let cl = &lay.crops;
            let per_row = cl.crops_per_row.max(1);
            let strip = cl.frames_per_crop * cl.frame_w;
            let src = IRect::new((data.sprite_index % per_row) * strip + frame * cl.frame_w, (data.sprite_index / per_row) * cl.frame_h, cl.frame_w, cl.frame_h);
            let mut s = Sprite::new(
                tex,
                src,
                FRect::new(pos.x, pos.y - TILE_F, TILE_F, TILE_F * 2.0),
                sort_depth((p.y * TILE + TILE - 8) as f32, map_h),
            );
            s.flip_x = c.flip;
            if c.dead {
                s.tint = Color::rgb(150, 120, 80);
            }
            out.sprite(s);
        }
    }

    // Placed objects (debris, forage).
    let shake_off = |shake: i32| if shake > 0 { ((shake as f32) * 1.3).sin() * 3.0 } else { 0.0 };
    for (p, o) in &loc.state.objects {
        let pos = cam.to_screen(Vec2::new((p.x * TILE) as f32, (p.y * TILE) as f32));
        let depth = sort_depth((p.y * TILE + TILE - 4) as f32, map_h);
        if let ObjKind::Ladder { down } = o.kind {
            let tex = rc.assets.ladder(rc.textures);
            out.draw(tex, IRect::new(if down { 0 } else { 16 }, 0, 16, 16), pos, ZOOM, DEPTH_GROUND + 0.002);
            continue;
        }
        if let ObjKind::Shaft = o.kind {
            // A dark hole with a lighter rim.
            let white = rc.assets.white(rc.textures);
            for (inset, h, col) in [(6.0, 40.0, Color::rgb(90, 70, 55)), (10.0, 32.0, Color::rgb(25, 18, 15)), (16.0, 22.0, Color::rgb(5, 4, 4))] {
                let r = FRect::new(pos.x + inset, pos.y + (TILE_F - h) / 2.0 + 4.0, TILE_F - inset * 2.0, h);
                let mut s = Sprite::new(white, IRect::new(0, 0, 1, 1), r, DEPTH_GROUND + 0.002 + inset * 0.00001);
                s.tint = col;
                out.sprite(s);
            }
            continue;
        }
        if let ObjKind::CrabPot { catch, .. } = &o.kind {
            let bob = ((w.game_ms as f32) / 500.0 + p.x as f32).sin() * 3.0;
            let (tex, src) = object_sprite(rc, w, &o.item_id);
            out.sprite(Sprite::new(tex, src, FRect::new(pos.x, pos.y + bob, TILE_F, TILE_F), DEPTH_GROUND + 0.003));
            if let Some(c) = catch {
                let (ctex, csrc) = item_sprite(rc, w, c);
                out.sprite(Sprite::new(ctex, csrc, FRect::new(pos.x + 16.0, pos.y - 30.0 + bob, 32.0, 32.0), DEPTH_SORT_MAX + 0.01));
            }
            continue;
        }
        if o.big {
            let (tex, src) = big_sprite(rc, w, &o.item_id);
            let mut s = Sprite::new(tex, src, FRect::new(pos.x + shake_off(o.shake), pos.y - TILE_F, TILE_F, TILE_F * 2.0), depth);
            s.flip_x = o.flip;
            out.sprite(s);
        } else {
            let (tex, src) = object_sprite(rc, w, &o.item_id);
            let mut s = Sprite::new(tex, src, FRect::new(pos.x + shake_off(o.shake), pos.y, TILE_F, TILE_F), depth);
            s.flip_x = o.flip;
            out.sprite(s);
        }
        // Finished machines show their product in a bobbing bubble.
        if let ObjKind::Machine { output: Some(item), ready: true, .. } = &o.kind {
            let bob = ((w.game_ms as f32) / 250.0).sin() * 4.0;
            let white = rc.assets.white(rc.textures);
            let bubble = FRect::new(pos.x + 4.0, pos.y - TILE_F * 1.9 + bob, TILE_F - 8.0, TILE_F - 8.0);
            out.sprite({
                let mut s = Sprite::new(white, IRect::new(0, 0, 1, 1), bubble, DEPTH_SORT_MAX + 0.01);
                s.tint = Color::rgba(255, 255, 255, 220);
                s
            });
            let (tex, src) = item_sprite(rc, w, item);
            out.sprite(Sprite::new(tex, src, FRect::new(bubble.x + 6.0, bubble.y + 6.0, bubble.w - 12.0, bubble.h - 12.0), DEPTH_SORT_MAX + 0.011));
        }
    }

    // Fruit trees: tree art tinted by species row, fruit drawn on the canopy.
    for (p, t) in &loc.state.terrain {
        let TerrainFeature::FruitTree(ft) = t else { continue };
        let tex = rc.assets.tex(rc.cm, rc.textures, &lay.tree_texture("fruit", season));
        let [cx, cy, cw, ch] = lay.trees.canopy;
        let base = cam.to_screen(Vec2::new((p.x * TILE) as f32, (p.y * TILE) as f32));
        let depth = sort_depth((p.y * TILE + TILE - 2) as f32, map_h);
        let growth = if ft.mature() { 1.0 } else { 0.3 + 0.6 * (1.0 - ft.days_until_mature as f32 / valley_sim::terrain::FRUIT_TREE_DAYS as f32) };
        let (wpx, hpx) = (cw as f32 * ZOOM * growth * 0.85, ch as f32 * ZOOM * growth * 0.85);
        let shake = shake_off(ft.shake);
        let canopy = FRect::new(base.x + TILE_F / 2.0 - wpx / 2.0 + shake, base.y + TILE_F - hpx, wpx, hpx);
        let row = w.fruit_trees.get(&ft.kind).map(|d| d.texture_sprite_row).unwrap_or(0);
        let tint = [Color::rgb(255, 235, 235), Color::rgb(235, 255, 235), Color::rgb(235, 235, 255), Color::rgb(255, 250, 220)][(row.rem_euclid(4)) as usize];
        let mut s = Sprite::new(tex, IRect::new(cx, cy, cw, ch), canopy, depth);
        s.tint = tint;
        out.sprite(s);
        for (k, id) in ft.fruit.iter().enumerate() {
            let (ftex, fsrc) = object_sprite(rc, w, id);
            let fx = canopy.x + canopy.w * (0.25 + 0.25 * k as f32);
            let fy = canopy.y + canopy.h * (0.25 + 0.1 * (k % 2) as f32);
            out.sprite(Sprite::new(ftex, fsrc, FRect::new(fx - 20.0, fy, 40.0, 40.0), depth + 0.0001));
        }
    }

    // Trees.
    for (p, t) in &loc.state.terrain {
        let TerrainFeature::Tree(tree) = t else { continue };
        let tex = rc.assets.tex(rc.cm, rc.textures, &lay.tree_texture(&tree.kind, season));
        let [cx, cy, cw, ch] = lay.trees.canopy;
        let [sx, sy, sw_, sh_] = lay.trees.stump;
        let canopy_src = IRect::new(cx, cy, cw, ch);
        let (cwf, chf) = (cw as f32 * ZOOM, ch as f32 * ZOOM);
        let base = cam.to_screen(Vec2::new((p.x * TILE) as f32, (p.y * TILE) as f32));
        let depth = sort_depth((p.y * TILE + TILE - 2) as f32, map_h);
        let shadow = rc.assets.shadow(rc.textures);
        out.draw_tinted(shadow, IRect::new(0, 0, 16, 6), Vec2::new(base.x, base.y + TILE_F - 20.0), ZOOM, Color::WHITE, depth - 0.0005);
        let canopy_rect = FRect::new(base.x + TILE_F / 2.0 - cwf / 2.0, base.y + TILE_F - chf, cwf, chf);
        if tree.stump || tree.falling > 0 {
            out.draw(tex, IRect::new(sx, sy, sw_, sh_), Vec2::new(base.x + TILE_F / 2.0 - sw_ as f32 * ZOOM / 2.0, base.y + TILE_F - sh_ as f32 * ZOOM), ZOOM, depth);
            if tree.falling > 0 {
                // Canopy tipping over.
                let mut s = Sprite::new(tex, canopy_src, canopy_rect, depth + 0.0001);
                s.origin = Vec2::new(cwf / 2.0, chf);
                let dir = if tree.flip { -1.0 } else { 1.0 };
                s.rotation = dir * (tree.falling as f32 / 60.0).powi(2) * std::f32::consts::FRAC_PI_2;
                s.tint = Color::WHITE.with_alpha(1.0 - tree.falling as f32 / 70.0);
                out.sprite(s);
            }
        } else if tree.mature() {
            let shake = shake_off(tree.shake);
            let mut r = canopy_rect;
            r.x += shake;
            let mut s = Sprite::new(tex, canopy_src, r, depth);
            s.flip_x = tree.flip;
            out.sprite(s);
        } else {
            // Saplings: scaled-down canopy.
            let scale = 0.25 + tree.stage as f32 * 0.15;
            let w_px = cwf * scale;
            let h_px = chf * scale;
            out.sprite(Sprite::new(tex, canopy_src, FRect::new(base.x + TILE_F / 2.0 - w_px / 2.0, base.y + TILE_F - h_px, w_px, h_px), depth));
        }
    }

    // Furniture: rugs lie on the floor, the rest is y-sorted by its footprint bottom.
    for f in &loc.state.furniture {
        let (tex, src) = furniture_sprite(rc, w, &f.id);
        let bottom = ((f.tile.y + f.footprint.1) * TILE) as f32;
        let p = cam.to_screen(Vec2::new((f.tile.x * TILE) as f32, bottom - src.h as f32 * ZOOM));
        let depth = if f.walkable { DEPTH_GROUND + 0.004 } else { sort_depth(bottom - 1.0, map_h) };
        out.sprite(Sprite::new(tex, src, FRect::new(p.x, p.y, src.w as f32 * ZOOM, src.h as f32 * ZOOM), depth));
    }

    // Farm buildings.
    if loc_name == "Farm" {
        for b in &w.buildings {
            let Some(data) = w.buildings_data.get(&b.kind) else { continue };
            let tex = rc.assets.tex(rc.cm, rc.textures, &data.texture.replace('\\', "/"));
            let (tw, th) = rc.textures.size(tex);
            let src = data.source_rect.as_ref().map(|r| IRect::new(r.x, r.y, r.width, r.height)).unwrap_or(IRect::new(0, 0, tw as i32, th as i32));
            let bottom = ((b.tile.y + b.size.1) * TILE) as f32;
            let left = (b.tile.x * TILE) as f32;
            let p = cam.to_screen(Vec2::new(left, bottom - src.h as f32 * ZOOM));
            let mut s = Sprite::new(tex, src, FRect::new(p.x, p.y, src.w as f32 * ZOOM, src.h as f32 * ZOOM), sort_depth(bottom - 1.0, map_h));
            if b.days_left > 0 {
                s.tint = Color::rgba(200, 200, 200, 140);
            }
            out.sprite(s);
            // Fish ponds: the resident fish and any produce waiting.
            if let Some(pond) = &b.pond {
                let center = cam.to_screen(Vec2::new((b.tile.x as f32 + b.size.0 as f32 / 2.0) * TILE_F, (b.tile.y as f32 + b.size.1 as f32 / 2.0) * TILE_F));
                if let Some(fish) = &pond.fish {
                    let (ftex, fsrc) = item_sprite(rc, w, &Item::object(fish, 1));
                    out.draw(ftex, fsrc, Vec2::new(center.x - 24.0, center.y - 16.0), 2.0, sort_depth(bottom, map_h) + 0.0001);
                    if let Some(ft) = rc.textures.id("__engine/font8x8") {
                        for (i, ch) in pond.population.to_string().chars().enumerate() {
                            let c = ch as i32;
                            out.draw_tinted(ft, IRect::new((c % 16) * 8, (c / 16) * 8, 8, 8), Vec2::new(center.x + 10.0 + i as f32 * 16.0, center.y - 8.0), 2.0, Color::WHITE, sort_depth(bottom, map_h) + 0.0002);
                        }
                    }
                }
                if let Some(item) = &pond.output {
                    let bob = ((w.game_ms as f32) / 300.0).sin() * 4.0;
                    let (itex, isrc) = item_sprite(rc, w, item);
                    let white = rc.assets.white(rc.textures);
                    let mut bubble = Sprite::new(white, IRect::new(0, 0, 1, 1), FRect::new(center.x - 26.0, center.y - 110.0 + bob, 52.0, 52.0), DEPTH_SORT_MAX + 0.01);
                    bubble.tint = Color::rgba(255, 255, 255, 220);
                    out.sprite(bubble);
                    out.sprite(Sprite::new(itex, isrc, FRect::new(center.x - 20.0, center.y - 104.0 + bob, 40.0, 40.0), DEPTH_SORT_MAX + 0.011));
                }
            }
        }
    }

    // Farm animals.
    for a in w.animals.iter().filter(|a| a.location == loc_name) {
        let Some(data) = w.animals_data.get(&a.kind) else { continue };
        let baby = a.is_baby(data);
        let tex_name = if baby { data.baby_texture.clone().unwrap_or_else(|| data.texture.clone()) } else { data.texture.clone() };
        let tex = rc.assets.tex(rc.cm, rc.textures, &tex_name.replace('\\', "/"));
        let (sw_, sh_) = (data.sprite_width.max(16), data.sprite_height.max(16));
        let frame = if a.moving { ((a.walk_ms / 200) % 4) as i32 } else { 0 };
        let row = match a.facing {
            Dir::Down => 0,
            Dir::Right => 1,
            Dir::Up => 2,
            Dir::Left => 3,
        };
        let src = IRect::new(frame * sw_, row * sh_, sw_, sh_);
        let scale = if baby { ZOOM * 0.75 } else { ZOOM };
        let (dw, dh) = (sw_ as f32 * scale, sh_ as f32 * scale);
        let p = cam.to_screen(a.pos);
        let depth = sort_depth(a.pos.y + TILE_F, map_h);
        let shadow = rc.assets.shadow(rc.textures);
        out.draw(shadow, IRect::new(0, 0, 16, 6), Vec2::new(p.x + TILE_F / 2.0 - dw / 2.0, p.y + TILE_F - 12.0), dw / 16.0, depth - 0.0005);
        out.sprite(Sprite::new(tex, src, FRect::new(p.x + TILE_F / 2.0 - dw / 2.0, p.y + TILE_F - dh, dw, dh), depth));
        if a.emote_ms > 0 {
            if let Some(ft) = rc.textures.id("__engine/font8x8") {
                out.draw_tinted(ft, IRect::new((3 % 16) * 8, 0, 8, 8), Vec2::new(p.x + 20.0, p.y + TILE_F - dh - 30.0), 3.0, Color::rgb(240, 60, 90), DEPTH_OVERLAY + 0.01);
            }
        }
    }

    // The traveling cart.
    if let Some(t) = w.cart_here(loc_name) {
        let tex = rc.assets.tex(rc.cm, rc.textures, "LooseSprites/Dev_Cart");
        let (tw, th) = rc.textures.size(tex);
        let p = cam.to_screen(Vec2::new(t.x as f32 * TILE_F, t.y as f32 * TILE_F));
        let bottom = (t.y + valley_sim::cart::CART_H) as f32 * TILE_F;
        let (dw, dh) = (tw as f32 * ZOOM, th as f32 * ZOOM);
        let left = p.x + (valley_sim::cart::CART_W as f32 * TILE_F - dw) / 2.0;
        let top = p.y + valley_sim::cart::CART_H as f32 * TILE_F - dh;
        out.sprite(Sprite::new(tex, IRect::new(0, 0, tw as i32, th as i32), FRect::new(left, top, dw, dh), sort_depth(bottom - 1.0, map_h)));
    }

    // Slingshot shots.
    for pr in w.projectiles.iter().filter(|p| p.location == loc_name) {
        let item = valley_sim::item::Item::object(&pr.item_id, 1);
        let (tex, src) = item_sprite(rc, w, &item);
        let p = cam.to_screen(pr.pos);
        let mut s = Sprite::new(tex, src, FRect::new(p.x - 16.0, p.y - 16.0, 32.0, 32.0), DEPTH_OVERLAY - 0.01);
        s.origin = Vec2::new(16.0, 16.0);
        s.rotation = pr.ms_left as f32 * 0.02;
        out.sprite(s);
    }

    // Children (and the crib while there's a baby).
    if loc_name == "FarmHouse" && !w.farmer.children.is_empty() {
        use valley_sim::children::ChildStage;
        let tex = rc.assets.tex(rc.cm, rc.textures, "Characters/Dev_Child");
        let today = w.clock.days_played();
        let crib = w.crib_tile();
        if w.farmer.children.iter().any(|c| matches!(c.stage(today), ChildStage::Newborn | ChildStage::Sitter)) {
            let p = cam.to_screen(Vec2::new(crib.x as f32 * TILE_F, crib.y as f32 * TILE_F));
            out.draw(tex, IRect::new(8 * 16, 0, 16, 16), p, ZOOM, sort_depth((crib.y + 1) as f32 * TILE_F - 2.0, map_h));
        }
        let shadow = rc.assets.shadow(rc.textures);
        for c in &w.farmer.children {
            let col = match c.stage(today) {
                ChildStage::Newborn => 0,
                ChildStage::Sitter => 1,
                ChildStage::Crawler => 2 + if c.moving { ((c.walk_ms / 250) % 2) as i32 } else { 0 },
                ChildStage::Toddler => 4 + if c.moving { ((c.walk_ms / 150) % 4) as i32 } else { 0 },
            };
            let p = cam.to_screen(c.pos);
            let in_crib = matches!(c.stage(today), ChildStage::Newborn | ChildStage::Sitter);
            // Babies lie in the crib, in front of its back rail.
            let depth = if in_crib { sort_depth((crib.y + 1) as f32 * TILE_F - 1.0, map_h) } else { sort_depth(c.pos.y + TILE_F, map_h) };
            if !in_crib {
                out.draw(shadow, IRect::new(0, 0, 16, 6), Vec2::new(p.x + 8.0, p.y + TILE_F - 12.0), 3.0, depth - 0.0005);
            }
            let mut s = Sprite::new(tex, IRect::new(col * 16, c.variant.clamp(0, 1) * 16, 16, 16), FRect::new(p.x, p.y, TILE_F, TILE_F), depth + 0.0001);
            s.flip_x = c.facing == Dir::Right;
            out.sprite(s);
        }
    }

    // Horses (the ridden one is drawn with the farmer).
    for (h, ridden) in w.horses_in(loc_name) {
        let tex = ["Animals/horse", "Animals/Dev_Horse"].iter().find(|t| rc.cm.exists(t)).map(|t| rc.assets.tex(rc.cm, rc.textures, t)).unwrap_or_else(|| rc.assets.tex(rc.cm, rc.textures, "Animals/Dev_Horse"));
        let frame = if h.moving { ((h.walk_ms / 120) % 4) as i32 } else { 0 };
        let row = match h.facing {
            Dir::Down => 0,
            Dir::Right => 1,
            Dir::Up => 2,
            Dir::Left => 3,
        };
        let (dw, dh) = (32.0 * ZOOM, 32.0 * ZOOM);
        let p = cam.to_screen(h.pos);
        // Ridden: just behind the farmer's sprite; otherwise sorted normally.
        let depth = sort_depth(h.pos.y + TILE_F, map_h) - if ridden { 0.0002 } else { 0.0 };
        let shadow = rc.assets.shadow(rc.textures);
        out.draw(shadow, IRect::new(0, 0, 16, 6), Vec2::new(p.x - 16.0, p.y + TILE_F - 12.0), 6.0, depth - 0.0005);
        out.sprite(Sprite::new(tex, IRect::new(frame * 32, row * 32, 32, 32), FRect::new(p.x + TILE_F / 2.0 - dw / 2.0, p.y + TILE_F - dh, dw, dh), depth));
    }

    // The farm pet.
    if let Some(pet) = w.pet.as_ref().filter(|p| p.location == loc_name) {
        let tex = rc.assets.tex(rc.cm, rc.textures, &pet.texture);
        let frame = if pet.moving { ((pet.walk_ms / 150) % 4) as i32 } else { 0 };
        let row = match pet.facing {
            Dir::Down => 0,
            Dir::Right => 1,
            Dir::Up => 2,
            Dir::Left => 3,
        };
        let (dw, dh) = (32.0 * ZOOM, 32.0 * ZOOM);
        let p = cam.to_screen(pet.pos);
        let depth = sort_depth(pet.pos.y + TILE_F, map_h);
        let shadow = rc.assets.shadow(rc.textures);
        out.draw(shadow, IRect::new(0, 0, 16, 6), Vec2::new(p.x + TILE_F / 2.0 - dw / 4.0, p.y + TILE_F - 12.0), dw / 32.0, depth - 0.0005);
        out.sprite(Sprite::new(tex, IRect::new(frame * 32, row * 32, 32, 32), FRect::new(p.x + TILE_F / 2.0 - dw / 2.0, p.y + TILE_F - dh, dw, dh), depth));
        if pet.emote_ms > 0 {
            if let Some(ft) = rc.textures.id("__engine/font8x8") {
                out.draw_tinted(ft, IRect::new(3 * 8, 0, 8, 8), Vec2::new(p.x + 20.0, p.y + TILE_F - dh + 10.0), 3.0, Color::rgb(240, 60, 90), DEPTH_OVERLAY + 0.01);
            }
        }
    }
    if loc_name == "Farm" && w.pet.as_ref().map(|p| p.bowl_filled).unwrap_or(false) {
        if let Some(t) = w.locations.get(loc_name).and_then(|l| l.map_prop_tile("PetBowlTile")) {
            let p = cam.to_screen(Vec2::new(t.x as f32 * TILE_F, t.y as f32 * TILE_F));
            let white = rc.assets.white(rc.textures);
            let mut s = Sprite::new(white, IRect::new(0, 0, 1, 1), FRect::new(p.x + 4.0 * ZOOM, p.y + 9.0 * ZOOM, 8.0 * ZOOM, 2.0 * ZOOM), DEPTH_GROUND + 0.001);
            s.tint = Color::rgba(130, 200, 255, 230);
            out.sprite(s);
        }
    }

    // Event actors take the place of NPCs and the farmer.
    let event_here = w.event.as_ref().filter(|e| e.location == loc_name);
    if let Some(e) = event_here {
        let shadow = rc.assets.shadow(rc.textures);
        for a in &e.actors {
            let tex = if a.texture.starts_with("__farmer/") { farmer_texture(rc, &w.farmer.appearance) } else { rc.assets.tex(rc.cm, rc.textures, &a.texture) };
            let frame = if a.moving.is_some() { ((a.walk_ms / 150) % 4) as i32 } else { 0 };
            let p = cam.to_screen(a.pos);
            let depth = sort_depth(a.pos.y + TILE_F, map_h);
            out.draw(shadow, IRect::new(0, 0, 16, 6), Vec2::new(p.x, p.y + TILE_F - 12.0), ZOOM, depth - 0.0005);
            out.draw(tex, character_src(a.facing, frame), Vec2::new(p.x, p.y - TILE_F), ZOOM, depth);
            if let Some((id, _)) = a.emote {
                if let Some(ft) = rc.textures.id("__engine/font8x8") {
                    let ch = match id {
                        8 | 16 => '?',
                        20 | 32 => '!',
                        12 => '*',
                        _ => '!',
                    } as i32;
                    let white = rc.assets.white(rc.textures);
                    out.draw_tinted(white, IRect::new(0, 0, 1, 1), Vec2::new(p.x + 14.0, p.y - 112.0), 36.0, Color::rgba(255, 255, 255, 230), DEPTH_OVERLAY + 0.009);
                    out.draw_tinted(ft, IRect::new((ch % 16) * 8, (ch / 16) * 8, 8, 8), Vec2::new(p.x + 20.0, p.y - 106.0), 3.0, Color::rgb(60, 40, 30), DEPTH_OVERLAY + 0.01);
                }
            }
        }
    }

    // NPCs.
    let shadow = rc.assets.shadow(rc.textures);
    for npc in w.npcs.iter().filter(|n| n.location == loc_name && event_here.is_none()) {
        let tex = rc.assets.tex(rc.cm, rc.textures, &npc.texture);
        let frame = walk_frame(npc.moving, npc.walk_ms, 200);
        let p = cam.to_screen(npc.pos);
        let depth = sort_depth(npc.pos.y + TILE_F, map_h);
        out.draw(shadow, IRect::new(0, 0, 16, 6), Vec2::new(p.x, p.y + TILE_F - 12.0), ZOOM, depth - 0.0005);
        let src = character_src_with(&lay.character, npc.facing, frame);
        let h = src.h as f32 * ZOOM;
        out.draw(tex, src, Vec2::new(p.x + TILE_F / 2.0 - src.w as f32 * ZOOM / 2.0, p.y + TILE_F - h), ZOOM, depth);
    }

    // Monsters.
    for m in w.monsters.iter().filter(|m| m.location == loc_name) {
        let tex = rc.assets.tex(rc.cm, rc.textures, &format!("Characters/Monsters/{}", m.info.name));
        let (tw, th) = rc.textures.size(tex);
        let fh = if th % 24 == 0 && th >= 24 { 24 } else { 16 };
        let frame = ((m.anim_ms / 150) % 4) as i32;
        let row = if m.movement == valley_sim::combat::Movement::Walker && th as i32 >= fh * 4 {
            match m.facing {
                Dir::Down => 0,
                Dir::Right => 1,
                Dir::Up => 2,
                Dir::Left => 3,
            }
        } else {
            0
        };
        let cols = (tw as i32 / 16).max(1);
        let src = IRect::new((frame % cols) * 16, row * fh, 16, fh);
        let p = cam.to_screen(m.pos);
        let depth = sort_depth(m.pos.y + TILE_F, map_h);
        out.draw(shadow, IRect::new(0, 0, 16, 6), Vec2::new(p.x, p.y + TILE_F - 12.0), ZOOM, depth - 0.0005);
        let h = fh as f32 * ZOOM;
        let mut s = Sprite::new(tex, src, FRect::new(p.x, p.y + TILE_F - h - m.z, TILE_F, h), depth);
        if m.invincible_ms > 0 && (m.invincible_ms / 50) % 2 == 0 {
            s.tint = Color::rgb(255, 120, 120);
        }
        s.flip_x = m.movement != valley_sim::combat::Movement::Walker && m.facing == Dir::Left;
        out.sprite(s);
    }

    // Farmer.
    if is_current && event_here.is_none() {
        draw_farmer(rc, out, w, cam, map_h);
    }

    // Floating damage numbers.
    if let Some(ft) = rc.textures.id("__engine/font8x8") {
        for f in w.floaters.iter().filter(|f| f.location == loc_name) {
            let p = cam.to_screen(f.pos);
            let alpha = (f.ms as f32 / 300.0).min(1.0);
            for (i, ch) in f.text.chars().enumerate() {
                let c = ch as i32;
                let src = IRect::new((c % 16) * 8, (c / 16) * 8, 8, 8);
                let pos = Vec2::new(p.x - f.text.len() as f32 * 12.0 + i as f32 * 24.0, p.y);
                out.draw_tinted(ft, src, pos + Vec2::new(3.0, 3.0), 3.0, Color::rgba(0, 0, 0, (160.0 * alpha) as u8), DEPTH_OVERLAY + 0.02);
                out.draw_tinted(ft, src, pos, 3.0, f.color.with_alpha(alpha), DEPTH_OVERLAY + 0.021);
            }
        }
    }

    // Dropped items.
    for d in &w.dropped {
        if d.location != loc_name {
            continue;
        }
        let (tex, src) = item_sprite(rc, w, &d.item);
        let bob = ((w.game_ms as f32 / 300.0) + d.pos.x).sin() * 2.0;
        let p = cam.to_screen(d.pos);
        let size = TILE_F * 0.75;
        out.draw(shadow, IRect::new(0, 0, 16, 6), Vec2::new(p.x - 20.0, p.y + 8.0), 2.5, sort_depth(d.pos.y, map_h));
        out.sprite(Sprite::new(tex, src, FRect::new(p.x - size / 2.0, p.y - size / 2.0 - d.z * 4.0 + bob - 8.0, size, size), sort_depth(d.pos.y + 1.0, map_h)));
    }

    if is_current {
        draw_fishing_world(rc, out, w, cam, map_h);
    }

    // Lit bombs and explosions.
    let white = rc.assets.white(rc.textures);
    for b in w.bombs.iter().filter(|b| b.location == loc_name) {
        let p = cam.to_screen(Vec2::new((b.tile.x * TILE) as f32, (b.tile.y * TILE) as f32));
        let flash = (b.fuse_ms / 120) % 2 == 0;
        let r = FRect::new(p.x + 16.0, p.y + 16.0, 32.0, 32.0);
        out.fill(r, if flash { Color::rgb(240, 70, 50) } else { Color::rgb(40, 40, 45) }, sort_depth((b.tile.y * TILE + TILE) as f32, map_h));
        out.fill(FRect::new(p.x + 30.0, p.y + 6.0, 4.0, 10.0), Color::rgb(250, 220, 90), sort_depth((b.tile.y * TILE + TILE) as f32, map_h) + 0.0001);
    }
    for e in w.explosions.iter().filter(|e| e.location == loc_name) {
        let c = cam.to_screen(e.center);
        let t = 1.0 - e.ms as f32 / 400.0;
        let rad = e.radius as f32 * TILE_F * (0.4 + 0.6 * t);
        for (k, col) in [(1.0f32, Color::rgba(255, 140, 40, 120)), (0.6, Color::rgba(255, 220, 120, 160)), (0.3, Color::rgba(255, 255, 230, 200))] {
            let rr = rad * k;
            out.draw_tinted(white, IRect::new(0, 0, 1, 1), Vec2::new(c.x - rr, c.y - rr), rr * 2.0, col.with_alpha(1.0 - t), DEPTH_OVERLAY + 0.003);
        }
    }

    draw_layer(rc, out, cam, loc, "Front", DEPTH_FRONT, season, ms);
    draw_layer(rc, out, cam, loc, "AlwaysFront", DEPTH_ALWAYS_FRONT, season, ms);

    draw_lighting(rc, out, w, loc, cam);
}

fn draw_farmer(rc: &mut RenderCtx, out: &mut DrawList, w: &World, cam: &Camera, map_h: f32) {
    let f = &w.farmer;
    let lay = rc.assets.layouts(rc.cm);
    let tex = match &lay.farmer.texture {
        Some(t) => rc.assets.tex(rc.cm, rc.textures, t),
        None => farmer_texture(rc, &w.worn_appearance()),
    };
    let shadow = rc.assets.shadow(rc.textures);
    let p = cam.to_screen(f.pos);
    let depth = sort_depth(f.pos.y + TILE_F, map_h);
    let mut frame = walk_frame(f.moving, f.walk_ms, 120);
    if let Some(u) = &f.tool_use {
        frame = if u.elapsed_ms < 150 { 1 } else { 3 };
    }
    out.draw(shadow, IRect::new(0, 0, 16, 6), Vec2::new(p.x, p.y + TILE_F - 12.0), ZOOM, depth - 0.0005);
    let blink = f.invincible_ms > 0 && (f.invincible_ms / 80) % 2 == 0;
    if !blink {
        // Riding: sit higher, legs still.
        let (lift, frame) = if f.mounted.is_some() { (40.0, 0) } else { (0.0, frame) };
        out.draw(tex, character_src(f.facing, frame), Vec2::new(p.x, p.y - TILE_F - lift), ZOOM, depth);
        if let Some(valley_sim::item::Item::Clothing { id, .. }) = &f.hat {
            draw_hat(rc, out, w, id, f.facing, Vec2::new(p.x, p.y - TILE_F - lift), frame, depth + 0.0001);
        }
    }

    // Charge meter above the head while charging an upgraded tool.
    if let Some(c) = &f.tool_charge {
        let white = rc.assets.white(rc.textures);
        let n = c.max_power + 1;
        for i in 0..n {
            let filled = i <= c.power();
            let col = if filled { Color::rgb(250, 220, 60) } else { Color::rgba(60, 40, 30, 200) };
            out.draw_tinted(white, IRect::new(0, 0, 1, 1), Vec2::new(p.x + 4.0 + i as f32 * 14.0, p.y - 92.0), 10.0, col, DEPTH_OVERLAY + 0.01);
        }
        let area = w.charged_area(f.facing_tile(), c.power());
        for t in area {
            let tp = cam.to_screen(Vec2::new((t.x * TILE) as f32, (t.y * TILE) as f32));
            out.outline(FRect::new(tp.x + 2.0, tp.y + 2.0, TILE_F - 4.0, TILE_F - 4.0), 3.0, Color::rgba(255, 255, 255, 140), DEPTH_OVERLAY + 0.005);
        }
    }

    // Weapon swing: the blade sweeps an arc in front of the farmer.
    if let Some(sw) = &w.weapon_swing {
        if let Some(item) = f.inventory.get(sw.slot) {
            let (ttex, src) = item_sprite(rc, w, item);
            let t = (sw.elapsed_ms as f32 / sw.total_ms as f32).clamp(0.0, 1.0);
            let base = match f.facing {
                Dir::Up => -std::f32::consts::FRAC_PI_2,
                Dir::Right => 0.0,
                Dir::Down => std::f32::consts::FRAC_PI_2,
                Dir::Left => std::f32::consts::PI,
            };
            let ang = base - 1.2 + 2.4 * t;
            let center = Vec2::new(p.x + TILE_F / 2.0, p.y + 16.0);
            let reach = 52.0;
            let tip = center + Vec2::new(ang.cos(), ang.sin()) * reach;
            let mut s = Sprite::new(ttex, src, FRect::new(tip.x - TILE_F / 2.0, tip.y - TILE_F / 2.0, TILE_F, TILE_F), if f.facing == Dir::Up { depth - 0.0002 } else { depth + 0.0002 });
            s.origin = Vec2::new(TILE_F / 2.0, TILE_F / 2.0);
            s.rotation = ang + std::f32::consts::FRAC_PI_4;
            out.sprite(s);
        }
    }

    // Weapon special moves and the slingshot.
    if let Some(mv) = &w.special {
        use valley_sim::arsenal::SpecialMove;
        let held = f.current_item().map(|i| item_sprite(rc, w, i));
        let center = Vec2::new(p.x + TILE_F / 2.0, p.y + 16.0);
        match mv {
            SpecialMove::Block { ms_left } => {
                // A shimmering guard in front of the farmer.
                if let Some((ttex, src)) = held {
                    let (dx, dy) = f.facing.delta();
                    let pos = center + Vec2::new(dx as f32 * 28.0, dy as f32 * 20.0);
                    let mut s = Sprite::new(ttex, src, FRect::new(pos.x - TILE_F / 2.0, pos.y - TILE_F / 2.0, TILE_F, TILE_F), depth + 0.0002);
                    s.origin = Vec2::new(TILE_F / 2.0, TILE_F / 2.0);
                    s.rotation = -std::f32::consts::FRAC_PI_4;
                    out.sprite(s);
                }
                let a = (120.0 * (*ms_left as f32 / valley_sim::arsenal::BLOCK_MS as f32)) as u8 + 40;
                out.outline(FRect::new(p.x - 4.0, p.y - 60.0, TILE_F + 8.0, TILE_F + 64.0), 3.0, Color::rgba(255, 255, 255, a), DEPTH_OVERLAY + 0.004);
            }
            SpecialMove::Flurry { next_ms, .. } => {
                if let Some((ttex, src)) = held {
                    let (dx, dy) = f.facing.delta();
                    let jab = 20.0 + (*next_ms as f32 / 120.0) * 24.0;
                    let pos = center + Vec2::new(dx as f32 * jab, dy as f32 * jab);
                    let ang = (dy as f32).atan2(dx as f32) + std::f32::consts::FRAC_PI_4;
                    let mut s = Sprite::new(ttex, src, FRect::new(pos.x - TILE_F / 2.0, pos.y - TILE_F / 2.0, TILE_F, TILE_F), if f.facing == Dir::Up { depth - 0.0002 } else { depth + 0.0002 });
                    s.origin = Vec2::new(TILE_F / 2.0, TILE_F / 2.0);
                    s.rotation = ang;
                    out.sprite(s);
                }
            }
            SpecialMove::Slam { windup_ms } => {
                if let Some((ttex, src)) = held {
                    let t = 1.0 - *windup_ms as f32 / valley_sim::arsenal::SLAM_WINDUP_MS as f32;
                    let pos = center + Vec2::new(0.0, -40.0 - 16.0 * t);
                    let mut s = Sprite::new(ttex, src, FRect::new(pos.x - TILE_F / 2.0, pos.y - TILE_F / 2.0, TILE_F, TILE_F), depth + 0.0002);
                    s.origin = Vec2::new(TILE_F / 2.0, TILE_F / 2.0);
                    s.rotation = -2.4 * t;
                    out.sprite(s);
                }
            }
        }
    }
    if let Some(pull) = &w.sling {
        if let Some(item) = f.inventory.get(pull.slot) {
            let (ttex, src) = item_sprite(rc, w, item);
            out.draw(ttex, src, Vec2::new(p.x + 8.0, p.y - 24.0), ZOOM * 0.75, depth + 0.0002);
        }
        // Dotted aim line, longer as the pull builds.
        let start = f.center() - Vec2::new(0.0, 24.0);
        let dir = match pull.aim {
            Some(a) if (a - start).length() > 4.0 => (a - start).normalized(),
            _ => {
                let (dx, dy) = f.facing.delta();
                Vec2::new(dx as f32, dy as f32)
            }
        };
        let power = pull.ms as f32 / valley_sim::arsenal::SLING_FULL_PULL_MS as f32;
        let white = rc.assets.white(rc.textures);
        for k in 1..=(2 + (power * 6.0) as i32) {
            let q = cam.to_screen(start + dir * (k as f32 * 28.0));
            out.draw_tinted(white, IRect::new(0, 0, 1, 1), Vec2::new(q.x - 3.0, q.y - 3.0), 6.0, Color::rgba(255, 255, 255, 170), DEPTH_OVERLAY + 0.003);
        }
    }

    // Tool swing.
    if let Some(u) = &f.tool_use {
        let item = f.inventory.get(u.slot);
        if let Some(item) = item {
            let (ttex, src) = item_sprite(rc, w, item);
            let t = (u.elapsed_ms as f32 / valley_sim::farmer::TOOL_TOTAL_MS as f32).clamp(0.0, 1.0);
            let hand = Vec2::new(p.x + TILE_F / 2.0, p.y + 8.0);
            let (start, end, behind) = match f.facing {
                Dir::Right => (-1.6f32, 0.6f32, false),
                Dir::Left => (1.6f32, -0.6f32, false),
                Dir::Up => (-0.6f32, 0.6f32, true),
                Dir::Down => (-0.9f32, 0.9f32, false),
            };
            let ang = if u.kind == ToolKind::WateringCan { 0.4 * (1.0 - t) } else { start + (end - start) * t.min(0.6) / 0.6 };
            let mut s = Sprite::new(ttex, src, FRect::new(hand.x - TILE_F / 2.0, hand.y - TILE_F, TILE_F, TILE_F), if behind { depth - 0.0002 } else { depth + 0.0002 });
            s.origin = Vec2::new(TILE_F / 2.0, TILE_F);
            s.rotation = ang;
            s.flip_x = f.facing == Dir::Left;
            if u.kind == ToolKind::WateringCan {
                let (dx, _) = f.facing.delta();
                s.dst.x += dx as f32 * 24.0;
            }
            out.sprite(s);
            if u.kind == ToolKind::WateringCan && u.applied {
                let white = rc.assets.white(rc.textures);
                let tp = cam.to_screen(Vec2::new((u.target.x * TILE) as f32, (u.target.y * TILE) as f32));
                for i in 0..6 {
                    let fx = tp.x + 12.0 + (i * 9 % 40) as f32;
                    let fy = tp.y + 10.0 + ((u.elapsed_ms as i32 / 20 + i * 7) % 40) as f32;
                    out.draw_tinted(white, IRect::new(0, 0, 1, 1), Vec2::new(fx, fy), 4.0, Color::rgba(140, 190, 255, 200), depth + 0.0003);
                }
            }
        }
    }
}

/// Darken outdoors at night and during rain; draw rain streaks.
fn draw_lighting(rc: &mut RenderCtx, out: &mut DrawList, w: &World, loc: &GameLocation, cam: &Camera) {
    let full = FRect::new(0.0, 0.0, cam.screen_w, cam.screen_h);
    let time = w.clock.time;
    // Approximate seasonal dusk.
    let dusk = match w.clock.season {
        valley_sim::time::Season::Winter => 1700,
        valley_sim::time::Season::Fall => 1800,
        _ => 1900,
    };
    let mut darkness = 0.0f32;
    if loc.outdoors && time >= dusk {
        let mins = valley_sim::time::Clock::minutes_between(dusk, time) as f32;
        let total = valley_sim::time::Clock::minutes_between(dusk, 2400) as f32;
        darkness = (mins / total).clamp(0.0, 1.0) * 0.75;
    }
    if loc.outdoors && w.weather.is_raining() {
        darkness = darkness.max(0.25);
    }
    if darkness > 0.0 {
        out.fill(full, Color::rgba(10, 10, 60, (darkness * 255.0) as u8), DEPTH_OVERLAY);
    }
    // Light pools at night: map "Light" property ("x y type ...") and lamp tiles.
    if darkness > 0.2 {
        let white = rc.assets.white(rc.textures);
        let mut lights: Vec<(i32, i32)> = Vec::new();
        if let Some(l) = loc.map.prop("Light") {
            let t: Vec<i32> = l.split_whitespace().filter_map(|v| v.parse().ok()).collect();
            for c in t.chunks(3) {
                if c.len() >= 2 {
                    lights.push((c[0], c[1]));
                }
            }
        }
        if let Some(b) = loc.map.layer("Buildings") {
            let (x0, y0, x1, y1) = cam.tile_range(TILE_F, 2);
            for y in y0.max(0)..=y1.min(b.height - 1) {
                for x in x0.max(0)..=x1.min(b.width - 1) {
                    if loc.map.tile_prop("Buildings", x, y, "Light").is_some_and(|v| valley_sim::location::truthy(&v)) || loc.map.tile_prop("Buildings", x, y, "Lamp").is_some() {
                        lights.push((x, y));
                    }
                }
            }
        }
        for (x, y) in lights {
            let c = cam.to_screen(Vec2::new(x as f32 * TILE_F + TILE_F / 2.0, y as f32 * TILE_F + TILE_F / 2.0));
            for (r, a) in [(200.0f32, 30u8), (140.0, 35), (80.0, 45)] {
                out.draw_tinted(white, IRect::new(0, 0, 1, 1), Vec2::new(c.x - r / 2.0, c.y - r / 2.0), r, Color::rgba(255, 220, 140, a), DEPTH_OVERLAY + 0.0005);
            }
        }
    }
    if loc.outdoors && w.weather == valley_sim::time::Weather::Snow {
        let white = rc.assets.white(rc.textures);
        let t = w.game_ms as f32;
        for i in 0..140 {
            let seed = (i * 7919) as f32;
            let x = (seed * 7.31 + t * 0.05 + (t / 900.0 + seed).sin() * 30.0).rem_euclid(cam.screen_w + 40.0) - 20.0;
            let y = (seed * 3.7 + t * 0.12).rem_euclid(cam.screen_h + 40.0) - 20.0;
            let size = 4.0 + (i % 3) as f32 * 2.0;
            out.draw_tinted(white, IRect::new(0, 0, 1, 1), Vec2::new(x, y), size, Color::rgba(255, 255, 255, 220), DEPTH_OVERLAY + 0.001);
        }
    }
    if loc.outdoors && w.weather == valley_sim::time::Weather::Wind {
        let white = rc.assets.white(rc.textures);
        let t = w.game_ms as f32;
        let leaf = match w.clock.season {
            valley_sim::time::Season::Fall => Color::rgb(220, 120, 40),
            valley_sim::time::Season::Spring => Color::rgb(250, 190, 210),
            _ => Color::rgb(120, 190, 80),
        };
        for i in 0..24 {
            let seed = (i * 4507) as f32;
            let x = (seed * 11.3 + t * 0.35).rem_euclid(cam.screen_w + 80.0) - 40.0;
            let y = (seed * 5.1 + t * 0.06 + (t / 300.0 + seed).sin() * 20.0).rem_euclid(cam.screen_h + 40.0) - 20.0;
            let mut s = Sprite::new(white, IRect::new(0, 0, 1, 1), FRect::new(x, y, 10.0, 6.0), DEPTH_OVERLAY + 0.001);
            s.tint = leaf;
            s.rotation = (t / 200.0 + seed).sin();
            out.sprite(s);
        }
    }
    if loc.outdoors && w.weather.is_raining() {
        let white = rc.assets.white(rc.textures);
        let t = w.game_ms as f32;
        for i in 0..90 {
            let seed = (i * 7919) as f32;
            let x = (seed * 13.37 + t * 0.6).rem_euclid(cam.screen_w + 40.0) - 20.0;
            let y = (seed * 3.1 + t * 1.4).rem_euclid(cam.screen_h + 40.0) - 20.0;
            let mut s = Sprite::new(white, IRect::new(0, 0, 1, 1), FRect::new(x, y, 2.0, 18.0), DEPTH_OVERLAY + 0.001);
            s.tint = Color::rgba(170, 190, 255, 150);
            s.rotation = 0.3;
            out.sprite(s);
        }
    }
}


/// Texture + source rect of an object id's icon.
pub fn object_sprite(rc: &mut RenderCtx, w: &World, id: &str) -> (engine::TexId, IRect) {
    let (tex_name, idx) = match w.items.object(id) {
        Some(o) => (o.texture(), o.data.sprite_index),
        None => ("Maps/springobjects".to_string(), 0),
    };
    let tex = rc.assets.tex(rc.cm, rc.textures, &tex_name);
    let (tw, _) = rc.textures.size(tex);
    let cols = (tw / 16).max(1) as i32;
    (tex, IRect::new((idx % cols) * 16, (idx / cols) * 16, 16, 16))
}

/// Texture + source rect of any item's 16x16 icon.
pub fn item_sprite(rc: &mut RenderCtx, w: &World, item: &Item) -> (engine::TexId, IRect) {
    match item {
        Item::Object { id, .. } => object_sprite(rc, w, id),
        Item::Big { id, .. } => big_sprite(rc, w, id),
        Item::Furniture { id } => furniture_sprite(rc, w, id),
        Item::Trinket { id } => {
            let (tex_name, idx) = match w.items.trinkets.get(id) {
                Some(t) => (t.texture.clone().unwrap_or_else(|| "TileSheets/Dev_Trinkets".into()).replace('\\', "/"), t.sheet_index),
                None => ("TileSheets/Dev_Trinkets".to_string(), 0),
            };
            let tex = rc.assets.tex(rc.cm, rc.textures, &tex_name);
            let (tw, _) = rc.textures.size(tex);
            let cols = (tw / 16).max(1) as i32;
            (tex, IRect::new((idx % cols) * 16, (idx / cols) * 16, 16, 16))
        }
        Item::Clothing { slot, id } => {
            let lay = rc.assets.layouts(rc.cm);
            let (tex_name, idx) = match w.items.clothing(*slot, id) {
                Some(c) => (c.texture.clone().unwrap_or_else(|| lay.clothing_icons.texture.clone()).replace('\\', "/"), c.sprite_index),
                None => (lay.clothing_icons.texture.clone(), 0),
            };
            let tex = rc.assets.tex(rc.cm, rc.textures, &tex_name);
            let (tw, _) = rc.textures.size(tex);
            let cols = (tw / 16).max(1) as i32;
            (tex, IRect::new((idx % cols) * 16, (idx / cols) * 16, 16, 16))
        }
        Item::Boots { id } => {
            let (tex_name, idx) = match w.items.boots.get(id) {
                Some(b) => (b.texture.clone().unwrap_or_else(|| "Maps/springobjects".into()).replace('\\', "/"), b.sprite_index),
                None => ("Maps/springobjects".to_string(), 0),
            };
            let tex = rc.assets.tex(rc.cm, rc.textures, &tex_name);
            let (tw, _) = rc.textures.size(tex);
            let cols = (tw / 16).max(1) as i32;
            (tex, IRect::new((idx % cols) * 16, (idx / cols) * 16, 16, 16))
        }
        Item::Weapon { id } => {
            let (tex_name, idx) = match w.items.weapons.get(id) {
                Some(d) => (if d.texture.is_empty() { "TileSheets/weapons".to_string() } else { d.texture.replace('\\', "/") }, d.sprite_index),
                None => ("TileSheets/weapons".to_string(), 0),
            };
            let tex = rc.assets.tex(rc.cm, rc.textures, &tex_name);
            let (tw, _) = rc.textures.size(tex);
            let cols = (tw / 16).max(1) as i32;
            (tex, IRect::new((idx % cols) * 16, (idx / cols) * 16, 16, 16))
        }
        Item::Tool(t) => {
            let (tex_name, idx) = match w.items.tools.get(t.kind.data_key()) {
                Some(d) => (if d.texture.is_empty() { "TileSheets/tools".to_string() } else { d.texture.replace('\\', "/") }, d.menu_sprite_index),
                None => ("TileSheets/tools".to_string(), 0),
            };
            let tex = rc.assets.tex(rc.cm, rc.textures, &tex_name);
            let (tw, _) = rc.textures.size(tex);
            let cols = (tw / 16).max(1) as i32;
            (tex, IRect::new((idx % cols) * 16, (idx / cols) * 16, 16, 16))
        }
    }
}

/// Texture + 16x32 source rect of a big craftable.
pub fn big_sprite(rc: &mut RenderCtx, w: &World, id: &str) -> (engine::TexId, IRect) {
    let (tex_name, idx) = match w.items.big(id) {
        Some(b) => (b.texture(), b.data.sprite_index),
        None => ("TileSheets/Craftables".to_string(), 0),
    };
    let tex = rc.assets.tex(rc.cm, rc.textures, &tex_name);
    let (tw, _) = rc.textures.size(tex);
    let cols = (tw / 16).max(1) as i32;
    (tex, IRect::new((idx % cols) * 16, (idx / cols) * 32, 16, 32))
}

/// Fishing line, bobber and bite marker (world space).
fn draw_fishing_world(rc: &mut RenderCtx, out: &mut DrawList, w: &World, cam: &Camera, map_h: f32) {
    use valley_sim::fishing::FishingState;
    let white = rc.assets.white(rc.textures);
    let f = &w.farmer;
    let p = cam.to_screen(f.pos);
    let depth = sort_depth(f.pos.y + TILE_F, map_h) + 0.0003;
    // Rod held up while fishing.
    if w.fishing.active() {
        if let Some(rod) = f.inventory.get(f.current_slot) {
            let (tex, src) = item_sprite(rc, w, rod);
            let mut s = Sprite::new(tex, src, FRect::new(p.x, p.y - 40.0, TILE_F, TILE_F), depth);
            s.flip_x = f.facing == Dir::Left;
            out.sprite(s);
        }
    }
    if let FishingState::Charging { power, .. } = &w.fishing {
        let bar = FRect::new(p.x + TILE_F + 8.0, p.y - TILE_F, 16.0, TILE_F * 1.5);
        out.fill(bar, Color::rgb(60, 40, 30), DEPTH_OVERLAY + 0.002);
        let h = (bar.h - 4.0) * power;
        let c = Color::rgb(230, 60, 40).lerp(Color::rgb(80, 210, 60), *power);
        out.fill(FRect::new(bar.x + 2.0, bar.y + bar.h - 2.0 - h, bar.w - 4.0, h), c, DEPTH_OVERLAY + 0.003);
    }
    if let Some(b) = w.bobber_pos() {
        let bp = cam.to_screen(b);
        let rod_tip = Vec2::new(p.x + TILE_F / 2.0 + f.facing.delta().0 as f32 * 40.0, p.y - 36.0);
        // Line as a series of dots.
        for i in 0..24 {
            let t = i as f32 / 23.0;
            let sag = (t * std::f32::consts::PI).sin() * 16.0;
            let q = rod_tip + (bp - rod_tip) * t;
            out.draw_tinted(white, IRect::new(0, 0, 1, 1), Vec2::new(q.x, q.y + sag), 2.0, Color::rgba(240, 240, 240, 200), depth);
        }
        let bob = if matches!(w.fishing, FishingState::Nibble { .. }) { ((w.game_ms / 60) % 2) as f32 * 4.0 } else { ((w.game_ms as f32) / 400.0).sin() * 2.0 };
        out.fill(FRect::new(bp.x - 6.0, bp.y - 6.0 + bob, 12.0, 12.0), Color::rgb(230, 50, 40), depth + 0.0001);
        out.fill(FRect::new(bp.x - 6.0, bp.y + bob, 12.0, 6.0), Color::rgb(245, 245, 245), depth + 0.0002);
        if matches!(w.fishing, FishingState::Nibble { .. }) {
            let font_tex = rc.textures.id("__engine/font8x8");
            if let Some(ft) = font_tex {
                out.draw_tinted(ft, IRect::new((33 % 16) * 8, (33 / 16) * 8, 8, 8), Vec2::new(p.x + 16.0, p.y - 120.0), 5.0, Color::YELLOW, DEPTH_OVERLAY + 0.01);
            }
        }
    }
}

/// The farmer's sprite sheet, generated from the original placeholder
/// character art recolored to the chosen appearance.
pub fn farmer_texture(rc: &mut RenderCtx, a: &valley_sim::farmer::Appearance) -> engine::TexId {
    let c = |v: [u8; 3]| Color::rgb(v[0], v[1], v[2]);
    let look = valley_content::dev::art::Look { skin: c(a.skin), hair: c(a.hair), shirt: c(a.shirt), legs: c(a.pants), long_hair: a.long_hair, beard: false };
    rc.assets.generated(rc.textures, &a.texture_name(), || valley_content::dev::art::character(&look))
}

/// A worn hat over a character sprite whose top-left is at `at`.
fn draw_hat(rc: &mut RenderCtx, out: &mut DrawList, w: &World, id: &str, facing: Dir, at: Vec2, frame: i32, depth: f32) {
    let lay = rc.assets.layouts(rc.cm).hats;
    let Some(hat) = w.items.hats.get(id) else { return };
    let tex = rc.assets.tex(rc.cm, rc.textures, &hat.texture.clone().unwrap_or(lay.texture.clone()).replace('\\', "/"));
    let (tw, _) = rc.textures.size(tex);
    let cols = (tw as i32 / lay.cell_w.max(1)).max(1);
    let i = hat.sprite_index.max(0);
    let row = lay.facing_rows[match facing {
        Dir::Down => 0,
        Dir::Right => 1,
        Dir::Up => 2,
        Dir::Left => 3,
    }];
    let src = IRect::new((i % cols) * lay.cell_w, ((i / cols) * 4 + row) * lay.cell_h, lay.cell_w, lay.cell_h);
    // Bob with the walk cycle like the head does.
    let bob = if frame % 2 == 1 { ZOOM } else { 0.0 };
    out.draw(tex, src, Vec2::new(at.x + lay.offset[0] as f32 * ZOOM, at.y + lay.offset[1] as f32 * ZOOM + bob), ZOOM, depth);
}

/// Texture + source rect of a furniture piece (sprite size from its data).
pub fn furniture_sprite(rc: &mut RenderCtx, w: &World, id: &str) -> (engine::TexId, IRect) {
    let (tex_name, idx, size) = match w.items.furniture.get(id) {
        Some(f) => (f.texture.clone().unwrap_or_else(|| "TileSheets/furniture".into()).replace('\\', "/"), f.sprite_index, f.sheet_size),
        None => ("TileSheets/furniture".to_string(), 0, (1, 1)),
    };
    let tex = rc.assets.tex(rc.cm, rc.textures, &tex_name);
    let (tw, _) = rc.textures.size(tex);
    let cols = (tw / 16).max(1) as i32;
    (tex, IRect::new((idx % cols) * 16, (idx / cols) * 16, size.0 * 16, size.1 * 16))
}
