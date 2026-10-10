//! The playable game as an engine [`GameModule`].

pub mod hud;
pub mod input_map;
pub mod menus;
pub mod saves;

use std::path::PathBuf;

use engine::font::BuiltinFont;
use engine::{Color, DrawList, EngineCtx, FRect, GameModule, IRect, Key, MouseButton, Vec2};
use valley_content::ContentManager;
use valley_render::ui::{self, Ui};
use valley_render::world_draw::{item_sprite, RenderCtx};
use valley_render::{Assets, Camera, DEPTH_HUD};
use valley_sim::inventory::TOOLBAR;
use valley_sim::save::SaveData;
use valley_sim::world::{NewGame, World, WorldEvent};
use valley_sim::{TilePos, TILE_F};

use crate::menus::{InvTab, Menu, QuestionKind, TABS, TAB_LABELS, TYPE_SPEED};

pub const MODULE_ID: &str = "valley";

#[derive(Clone, Debug, PartialEq)]
enum Field {
    Name,
    Farm,
    Favorite,
}

enum Screen {
    Title { hovered: Option<usize> },
    LoadList { selected: usize },
    Create { name: String, farm: String, favorite: String, male: bool, field: Field },
    Playing,
}

/// Fade-to-black transition.
#[derive(Default)]
struct Fade {
    /// 0 = clear, 1 = black.
    alpha: f32,
    target: f32,
}

pub struct ValleyGame {
    pub cm: ContentManager,
    pub world: Option<World>,
    assets: Assets,
    font: Option<BuiltinFont>,
    screen: Screen,
    menus: Vec<Menu>,
    toasts: Vec<(String, u32)>,
    fade: Fade,
    pub save_dir: PathBuf,
    /// Fixed seed for new games (tests/replays); None = derive from time.
    pub new_game_seed: Option<u64>,
    cursor_tile: Option<TilePos>,
    pub last_save: Option<PathBuf>,
}

impl ValleyGame {
    pub fn new(cm: ContentManager, save_dir: PathBuf) -> Self {
        Self {
            cm,
            world: None,
            assets: Assets::new(),
            font: None,
            screen: Screen::Title { hovered: None },
            menus: Vec::new(),
            toasts: Vec::new(),
            fade: Fade::default(),
            save_dir,
            new_game_seed: None,
            cursor_tile: None,
            last_save: None,
        }
    }

    /// Skip the title screen and start a new game immediately.
    pub fn start_new_game(&mut self, opts: NewGame) {
        let w = World::new(&mut self.cm, opts);
        self.world = Some(w);
        self.screen = Screen::Playing;
        self.menus.clear();
        self.fade = Fade { alpha: 1.0, target: 0.0 };
    }

    pub fn load_game(&mut self, data: SaveData) {
        let w = data.restore(&mut self.cm);
        self.world = Some(w);
        self.screen = Screen::Playing;
        self.menus.clear();
        self.fade = Fade { alpha: 1.0, target: 0.0 };
    }

    pub fn is_playing(&self) -> bool {
        matches!(self.screen, Screen::Playing)
    }

    pub fn menu_open(&self) -> bool {
        !self.menus.is_empty()
    }

    fn toast(&mut self, msg: impl Into<String>) {
        self.toasts.push((msg.into(), 3500));
        if self.toasts.len() > 8 {
            self.toasts.remove(0);
        }
    }

    fn save(&mut self) {
        let Some(w) = &self.world else { return };
        let data = SaveData::capture(w);
        match saves::write(&self.save_dir, &data) {
            Ok(p) => self.last_save = Some(p),
            Err(e) => self.toast(format!("Save failed: {e}")),
        }
    }

    fn camera(&self, ctx: &EngineCtx) -> Option<Camera> {
        let w = self.world.as_ref()?;
        let loc = w.location();
        Some(Camera::follow(
            w.farmer.center(),
            loc.width() as f32 * TILE_F,
            loc.height() as f32 * TILE_F,
            ctx.screen_w as f32,
            ctx.screen_h as f32,
        ))
    }

    // ------------------------------------------------------------------ update

    fn update_title(&mut self, ctx: &mut EngineCtx) {
        let (sw, sh) = (ctx.screen_w as f32, ctx.screen_h as f32);
        let buttons = title_buttons(sw, sh);
        let mouse = ctx.input.mouse;
        let hovered = buttons.iter().position(|b| b.contains(mouse));
        self.screen = Screen::Title { hovered };
        let click = ctx.input.mouse_pressed(MouseButton::Left);
        let choice = if click { hovered } else if ctx.input.pressed(Key::N) || ctx.input.pressed(Key::Enter) { Some(0) } else if ctx.input.pressed(Key::Escape) { Some(2) } else { None };
        match choice {
            Some(0) => {
                self.screen = Screen::Create { name: String::new(), farm: String::new(), favorite: String::new(), male: true, field: Field::Name };
            }
            Some(1) => self.screen = Screen::LoadList { selected: 0 },
            Some(2) => ctx.quit_requested = true,
            _ => {}
        }
    }

    fn update_load_list(&mut self, ctx: &mut EngineCtx) {
        let list = saves::list(&self.save_dir);
        let Screen::LoadList { selected } = &mut self.screen else { return };
        if ctx.input.pressed(Key::Escape) {
            self.screen = Screen::Title { hovered: None };
            return;
        }
        if list.is_empty() {
            if ctx.input.any_pressed() {
                self.screen = Screen::Title { hovered: None };
            }
            return;
        }
        if ctx.input.pressed(Key::Down) || ctx.input.pressed(Key::S) {
            *selected = (*selected + 1) % list.len();
        }
        if ctx.input.pressed(Key::Up) || ctx.input.pressed(Key::W) {
            *selected = (*selected + list.len() - 1) % list.len();
        }
        let (sw, _) = (ctx.screen_w as f32, ctx.screen_h as f32);
        let mut clicked = None;
        for i in 0..list.len() {
            if load_row_rect(sw, i).contains(ctx.input.mouse) {
                *selected = i;
                if ctx.input.mouse_pressed(MouseButton::Left) {
                    clicked = Some(i);
                }
            }
        }
        let pick = clicked.or(if ctx.input.pressed(Key::Enter) { Some(*selected) } else { None });
        if let Some(i) = pick {
            match saves::load(&list[i].path) {
                Ok(d) => self.load_game(d),
                Err(e) => self.toast(format!("Load failed: {e}")),
            }
        }
    }

    fn update_create(&mut self, ctx: &mut EngineCtx) {
        let Screen::Create { name, farm, favorite, male, field } = &mut self.screen else { return };
        if ctx.input.pressed(Key::Escape) {
            self.screen = Screen::Title { hovered: None };
            return;
        }
        let target = match field {
            Field::Name => &mut *name,
            Field::Farm => &mut *farm,
            Field::Favorite => &mut *favorite,
        };
        for c in ctx.input.text.chars() {
            if (c.is_ascii_graphic() || c == ' ') && target.len() < 16 {
                target.push(c);
            }
        }
        if ctx.input.pressed(Key::Backspace) {
            target.pop();
        }
        if ctx.input.pressed(Key::Tab) {
            *field = match field {
                Field::Name => Field::Farm,
                Field::Farm => Field::Favorite,
                Field::Favorite => Field::Name,
            };
        }
        let (sw, sh) = (ctx.screen_w as f32, ctx.screen_h as f32);
        let rects = create_rects(sw, sh);
        if ctx.input.mouse_pressed(MouseButton::Left) {
            let m = ctx.input.mouse;
            if rects[0].contains(m) {
                *field = Field::Name;
            } else if rects[1].contains(m) {
                *field = Field::Farm;
            } else if rects[2].contains(m) {
                *field = Field::Favorite;
            } else if rects[3].contains(m) {
                *male = !*male;
            }
        }
        let ok = ctx.input.pressed(Key::Enter) || (ctx.input.mouse_pressed(MouseButton::Left) && rects[4].contains(ctx.input.mouse));
        if ok && !name.trim().is_empty() {
            let seed = self.new_game_seed.unwrap_or_else(|| {
                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(1)
            });
            let opts = NewGame {
                farmer_name: name.trim().to_string(),
                farm_name: if farm.trim().is_empty() { "Valley".into() } else { farm.trim().to_string() },
                favorite_thing: favorite.trim().to_string(),
                male: *male,
                seed,
            };
            self.start_new_game(opts);
            self.save();
        }
    }

    fn update_playing(&mut self, ctx: &mut EngineCtx) {
        let cam = self.camera(ctx);
        self.cursor_tile = cam.map(|c| TilePos::from_world(c.to_world(ctx.input.mouse)));

        if !self.menus.is_empty() {
            self.update_menu(ctx);
        } else if input_map::menu_pressed(&ctx.input) {
            self.menus.push(Menu::Inventory { tab: InvTab::Items, held: None, hovered: None });
        } else if let Some(w) = self.world.as_mut() {
            // Toolbar clicks select slots instead of using tools.
            let mut input = input_map::player_input(&ctx.input, self.cursor_tile);
            if ctx.input.mouse_pressed(MouseButton::Left) {
                let (sw, sh) = (ctx.screen_w as f32, ctx.screen_h as f32);
                if let Some(i) = (0..TOOLBAR).find(|&i| hud::toolbar_slot_rect(sw, sh, i).contains(ctx.input.mouse)) {
                    input.use_tool = false;
                    input.select_slot = Some(i);
                }
            }
            w.update(&input);
        }
        if let Some(w) = self.world.as_mut() {
            w.paused = !self.menus.is_empty();
            if w.paused {
                // Keep animations running while paused.
                w.update(&Default::default());
            }
        }
        self.handle_world_events(ctx);

        for t in self.toasts.iter_mut() {
            t.1 = t.1.saturating_sub(16);
        }
        self.toasts.retain(|t| t.1 > 0);
    }

    fn handle_world_events(&mut self, ctx: &mut EngineCtx) {
        let Some(w) = self.world.as_mut() else { return };
        let events = w.drain_events();
        for e in events {
            match e {
                WorldEvent::Sound(cue) => ctx.audio.play(&cue),
                WorldEvent::Hud(m) => self.toast(m),
                WorldEvent::Pickup(item) => {
                    let name = self.world.as_ref().map(|w| w.items.display_name(&item)).unwrap_or_default();
                    let n = item.stack();
                    self.toast(if n > 1 { format!("+{n} {name}") } else { format!("+ {name}") });
                }
                WorldEvent::Dialogue { npc, pages } => self.menus.push(Menu::Dialogue { npc, pages, page: 0, shown: 0.0 }),
                WorldEvent::Message(text) => self.menus.push(Menu::Message { text, shown: 0.0 }),
                WorldEvent::OpenShop(stock) => self.menus.push(Menu::Shop { stock, scroll: 0, hovered: None }),
                WorldEvent::Warped { .. } => self.fade = Fade { alpha: 1.0, target: 0.0 },
                WorldEvent::SleepPrompt => self.menus.push(Menu::Question {
                    text: "Go to sleep for the night?".into(),
                    options: vec!["Yes".into(), "No".into()],
                    selected: 0,
                    kind: QuestionKind::Sleep,
                }),
                WorldEvent::EatPrompt { slot } => {
                    let name = self.world.as_ref().and_then(|w| w.farmer.inventory.get(slot).map(|i| w.items.display_name(i))).unwrap_or_default();
                    self.menus.push(Menu::Question { text: format!("Eat {name}?"), options: vec!["Yes".into(), "No".into()], selected: 0, kind: QuestionKind::Eat { slot } });
                }
                WorldEvent::DayEnded(report) => {
                    self.menus.clear();
                    self.menus.push(Menu::DayEnd { report, ms: 0 });
                    self.save();
                }
                WorldEvent::LevelUp(skill, level) => self.toast(format!("{skill:?} level {level}!")),
                WorldEvent::Exhausted => self.toast("You're exhausted..."),
                WorldEvent::OpenChest(tile) => self.menus.push(Menu::Chest { tile, hovered: None }),
            }
            if self.world.is_none() {
                break;
            }
        }
    }

    fn update_menu(&mut self, ctx: &mut EngineCtx) {
        let (sw, sh) = (ctx.screen_w as f32, ctx.screen_h as f32);
        let Some(menu) = self.menus.last_mut() else { return };
        let confirm = input_map::confirm_pressed(&ctx.input);
        let mut close = false;
        let mut action: Option<Box<dyn FnOnce(&mut ValleyGame, &mut EngineCtx)>> = None;
        match menu {
            Menu::Dialogue { pages, page, shown, .. } => {
                let len = pages[*page].text.chars().count() as f32;
                *shown = (*shown + TYPE_SPEED).min(len);
                if confirm {
                    if *shown < len {
                        *shown = len;
                    } else if *page + 1 < pages.len() {
                        *page += 1;
                        *shown = 0.0;
                    } else {
                        close = true;
                    }
                }
            }
            Menu::Message { text, shown } => {
                let len = text.chars().count() as f32;
                *shown = (*shown + TYPE_SPEED * 2.0).min(len);
                if confirm {
                    if *shown < len {
                        *shown = len;
                    } else {
                        close = true;
                    }
                }
            }
            Menu::Question { options, selected, kind, .. } => {
                if ctx.input.pressed(Key::Down) || ctx.input.pressed(Key::S) {
                    *selected = (*selected + 1) % options.len();
                }
                if ctx.input.pressed(Key::Up) || ctx.input.pressed(Key::W) {
                    *selected = (*selected + options.len() - 1) % options.len();
                }
                let rects = question_option_rects(sw, sh, options.len());
                let mut chosen = None;
                for (i, r) in rects.iter().enumerate() {
                    if r.contains(ctx.input.mouse) {
                        *selected = i;
                        if ctx.input.mouse_pressed(MouseButton::Left) {
                            chosen = Some(i);
                        }
                    }
                }
                if ctx.input.pressed(Key::Enter) || ctx.input.pressed(Key::Space) || ctx.input.pressed(Key::X) || ctx.input.pressed(Key::C) {
                    chosen = Some(*selected);
                }
                if ctx.input.pressed(Key::Escape) {
                    chosen = Some(options.len() - 1);
                }
                if let Some(i) = chosen {
                    close = true;
                    let kind = kind.clone();
                    if i == 0 {
                        action = Some(Box::new(move |g: &mut ValleyGame, _ctx: &mut EngineCtx| match kind {
                            QuestionKind::Sleep => {
                                g.fade = Fade { alpha: 0.0, target: 1.0 };
                                if let Some(w) = g.world.as_mut() {
                                    w.end_day(false);
                                }
                            }
                            QuestionKind::Eat { slot } => {
                                if let Some(w) = g.world.as_mut() {
                                    w.eat(slot);
                                }
                            }
                            QuestionKind::QuitToTitle => {
                                g.save();
                                g.world = None;
                                g.menus.clear();
                                g.screen = Screen::Title { hovered: None };
                            }
                        }));
                    }
                }
            }
            Menu::Shop { stock, scroll, hovered } => {
                let rows = shop_rows(sw, sh);
                let visible = rows.len();
                if ctx.input.wheel < 0.0 || ctx.input.pressed(Key::Down) {
                    *scroll = (*scroll + 1).min(stock.entries.len().saturating_sub(visible));
                }
                if ctx.input.wheel > 0.0 || ctx.input.pressed(Key::Up) {
                    *scroll = scroll.saturating_sub(1);
                }
                *hovered = rows.iter().position(|r| r.contains(ctx.input.mouse)).map(|i| i + *scroll).filter(|&i| i < stock.entries.len());
                if ctx.input.mouse_pressed(MouseButton::Left) {
                    if let Some(i) = *hovered {
                        let entry = stock.entries[i].clone();
                        let n = if ctx.input.held(Key::LeftShift) { 5 } else if ctx.input.held(Key::LeftCtrl) { 25 } else { 1 };
                        action = Some(Box::new(move |g: &mut ValleyGame, _| {
                            if let Some(w) = g.world.as_mut() {
                                if !w.buy(&entry, n) && entry.price * n > w.farmer.money {
                                    g.toast("Not enough money.");
                                }
                            }
                        }));
                    } else {
                        // Selling from the inventory strip.
                        let slots = shop_inventory_rects(sw, sh);
                        if let Some(i) = slots.iter().position(|r| r.contains(ctx.input.mouse)) {
                            action = Some(Box::new(move |g: &mut ValleyGame, _| {
                                if let Some(w) = g.world.as_mut() {
                                    w.sell_slot(i);
                                }
                            }));
                        }
                    }
                }
                if ctx.input.pressed(Key::Escape) || ctx.input.pressed(Key::E) || ctx.input.mouse_pressed(MouseButton::Right) {
                    close = true;
                }
            }
            Menu::Inventory { tab, held, hovered } => {
                let tabs = inventory_tab_rects(sw, sh);
                if ctx.input.mouse_pressed(MouseButton::Left) {
                    if let Some(i) = tabs.iter().position(|r| r.contains(ctx.input.mouse)) {
                        *tab = TABS[i].clone();
                    }
                }
                match tab {
                    InvTab::Items => {
                        let slots = inventory_slot_rects(sw, sh);
                        *hovered = slots.iter().position(|r| r.contains(ctx.input.mouse));
                        if ctx.input.mouse_pressed(MouseButton::Left) {
                            if let Some(i) = *hovered {
                                match *held {
                                    None => {
                                        if self.world.as_ref().is_some_and(|w| w.farmer.inventory.get(i).is_some()) {
                                            *held = Some(i);
                                        }
                                    }
                                    Some(h) => {
                                        if let Some(w) = self.world.as_mut() {
                                            w.farmer.inventory.swap(h, i);
                                        }
                                        *held = None;
                                    }
                                }
                            } else if let Some(trash) = trash_rect(sw, sh).filter(|r| r.contains(ctx.input.mouse)) {
                                let _ = trash;
                                if let (Some(h), Some(w)) = (*held, self.world.as_mut()) {
                                    if w.farmer.inventory.get(h).and_then(|i| i.tool()).is_none() {
                                        w.farmer.inventory.take(h);
                                    }
                                    *held = None;
                                }
                            }
                        }
                    }
                    InvTab::Crafting => {
                        let names = self.world.as_ref().map(known_recipes).unwrap_or_default();
                        let rects = recipe_rects(sw, sh);
                        *hovered = rects.iter().position(|r| r.contains(ctx.input.mouse)).filter(|&i| i < names.len());
                        if ctx.input.mouse_pressed(MouseButton::Left) {
                            if let Some(i) = *hovered {
                                let name = names[i].clone();
                                action = Some(Box::new(move |g: &mut ValleyGame, _| {
                                    if let Some(w) = g.world.as_mut() {
                                        w.craft(&name);
                                    }
                                }));
                            }
                        }
                    }
                    InvTab::Options => {
                        let b = options_buttons(sw, sh);
                        if ctx.input.mouse_pressed(MouseButton::Left) {
                            if b[0].contains(ctx.input.mouse) {
                                action = Some(Box::new(|g: &mut ValleyGame, _| {
                                    g.menus.clear();
                                    g.menus.push(Menu::Question {
                                        text: "Save and return to the title screen? (Progress is saved each night.)".into(),
                                        options: vec!["Yes".into(), "No".into()],
                                        selected: 1,
                                        kind: QuestionKind::QuitToTitle,
                                    });
                                }));
                            } else if b[1].contains(ctx.input.mouse) {
                                action = Some(Box::new(|_g: &mut ValleyGame, ctx: &mut EngineCtx| ctx.quit_requested = true));
                            }
                        }
                    }
                    _ => {}
                }
                if input_map::menu_pressed(&ctx.input) {
                    close = true;
                }
            }
            Menu::Chest { tile, hovered } => {
                let (prow, crow) = chest_rects(sw, sh);
                *hovered = crow.iter().position(|r| r.contains(ctx.input.mouse)).map(|i| (false, i)).or_else(|| prow.iter().position(|r| r.contains(ctx.input.mouse)).map(|i| (true, i)));
                if ctx.input.mouse_pressed(MouseButton::Left) {
                    if let Some((from_player, i)) = *hovered {
                        let t = *tile;
                        action = Some(Box::new(move |g: &mut ValleyGame, _| {
                            if let Some(w) = g.world.as_mut() {
                                w.chest_transfer(t, from_player, i);
                            }
                        }));
                    }
                }
                if input_map::menu_pressed(&ctx.input) || ctx.input.mouse_pressed(MouseButton::Right) {
                    close = true;
                }
            }
            Menu::DayEnd { ms, .. } => {
                *ms += 16;
                if (*ms > 600 && confirm) || *ms > 15000 {
                    close = true;
                    action = Some(Box::new(|g: &mut ValleyGame, _| g.fade = Fade { alpha: 1.0, target: 0.0 }));
                }
            }
        }
        if close {
            self.menus.pop();
        }
        if let Some(a) = action {
            a(self, ctx);
        }
    }

    // ------------------------------------------------------------------ draw

    fn draw_title(&mut self, ctx: &mut EngineCtx, out: &mut DrawList) {
        let (sw, sh) = (ctx.screen_w as f32, ctx.screen_h as f32);
        out.clear(Color::rgb(120, 180, 230));
        // Simple original backdrop: sky gradient, hills, title.
        for i in 0..12 {
            let t = i as f32 / 12.0;
            out.fill(FRect::new(0.0, sh * t * 0.6, sw, sh * 0.06), Color::rgb(120, 180, 230).lerp(Color::rgb(250, 220, 180), t), 0.0);
        }
        out.fill(FRect::new(0.0, sh * 0.6, sw, sh * 0.4), Color::rgb(96, 170, 64), 0.01);
        out.fill(FRect::new(0.0, sh * 0.58, sw, sh * 0.04), Color::rgb(80, 150, 56), 0.011);
        let font = self.font.as_ref().unwrap();
        let white = self.assets.white(&mut ctx.textures);
        let mut ui = Ui { out, font, white, depth: DEPTH_HUD };
        ui.text_shadow("valley-rs", Vec2::new(sw / 2.0 - 9.0 * 4.0 * 8.0 / 2.0, sh * 0.15), 8.0, ui::TEXT_LIGHT);
        let sub = if self.cm.has_game_content { "using your game content" } else { "placeholder content (no game folder set)" };
        ui.text_centered(sub, sw / 2.0, sh * 0.15 + 80.0, 2.0, ui::TEXT_LIGHT);
        let hovered = match &self.screen {
            Screen::Title { hovered } => *hovered,
            _ => None,
        };
        for (i, (r, label)) in title_buttons(sw, sh).iter().zip(["New", "Load", "Exit"]).enumerate() {
            ui.button(*r, label, hovered == Some(i));
        }
    }

    fn draw_load_list(&mut self, ctx: &mut EngineCtx, out: &mut DrawList) {
        let (sw, sh) = (ctx.screen_w as f32, ctx.screen_h as f32);
        out.clear(Color::rgb(60, 90, 60));
        let list = saves::list(&self.save_dir);
        let sel = match &self.screen {
            Screen::LoadList { selected } => *selected,
            _ => 0,
        };
        let font = self.font.as_ref().unwrap();
        let white = self.assets.white(&mut ctx.textures);
        let mut ui = Ui { out, font, white, depth: DEPTH_HUD };
        ui.panel(FRect::new(sw / 2.0 - 400.0, 40.0, 800.0, sh - 80.0));
        ui.text_centered("Load Game", sw / 2.0, 70.0, 3.0, ui::TEXT);
        if list.is_empty() {
            ui.text_centered("No saved games yet.", sw / 2.0, 160.0, 2.0, ui::TEXT);
        }
        for (i, s) in list.iter().enumerate() {
            let r = load_row_rect(sw, i);
            ui.slot(r, i == sel);
            ui.text(&format!("{} ({} Farm)", s.farmer, s.farm), Vec2::new(r.x + 16.0, r.y + 12.0), 2.0, ui::TEXT);
            ui.text(&format!("{}   {}g", s.day, s.money), Vec2::new(r.x + 16.0, r.y + 40.0), 2.0, ui::TEXT);
        }
        let _ = sh;
    }

    fn draw_create(&mut self, ctx: &mut EngineCtx, out: &mut DrawList) {
        let (sw, sh) = (ctx.screen_w as f32, ctx.screen_h as f32);
        out.clear(Color::rgb(60, 90, 60));
        let Screen::Create { name, farm, favorite, male, field } = &self.screen else { return };
        let font = self.font.as_ref().unwrap();
        let white = self.assets.white(&mut ctx.textures);
        let mut ui = Ui { out, font, white, depth: DEPTH_HUD };
        ui.panel(FRect::new(sw / 2.0 - 360.0, sh / 2.0 - 260.0, 720.0, 520.0));
        ui.text_centered("New Farmer", sw / 2.0, sh / 2.0 - 230.0, 3.0, ui::TEXT);
        let rects = create_rects(sw, sh);
        let caret = if (ctx.tick / 30) % 2 == 0 { "_" } else { " " };
        for (i, (label, value, f)) in [("Name", name, Field::Name), ("Farm Name", farm, Field::Farm), ("Favorite Thing", favorite, Field::Favorite)].into_iter().enumerate() {
            let r = rects[i];
            ui.text(label, Vec2::new(r.x, r.y - 22.0), 2.0, ui::TEXT);
            ui.slot(r, *field == f);
            let shown = if *field == f { format!("{value}{caret}") } else { value.clone() };
            ui.text(&shown, Vec2::new(r.x + 12.0, r.y + 14.0), 2.0, ui::TEXT);
        }
        ui.button(rects[3], if *male { "Male" } else { "Female" }, false);
        ui.button(rects[4], "OK", !name.trim().is_empty());
        ui.text_centered("Tab: next field   Enter: start   Esc: back", sw / 2.0, sh / 2.0 + 220.0, 2.0, ui::TEXT);
    }

    fn draw_playing(&mut self, ctx: &mut EngineCtx, out: &mut DrawList) {
        let (sw, sh) = (ctx.screen_w as f32, ctx.screen_h as f32);
        out.clear(Color::BLACK);
        let Some(cam) = self.camera(ctx) else { return };
        let Some(w) = self.world.as_ref() else { return };
        let mut rc = RenderCtx { cm: &mut self.cm, textures: &mut ctx.textures, assets: &mut self.assets };
        valley_render::draw_world(&mut rc, out, w, &cam);

        // Cursor highlight on the targeted tile.
        if self.menus.is_empty() {
            let target = w.target_tile(self.cursor_tile);
            let p = cam.to_screen(Vec2::new(target.x as f32 * TILE_F, target.y as f32 * TILE_F));
            if w.farmer.current_item().is_some() {
                out.outline(FRect::new(p.x, p.y, TILE_F, TILE_F), 2.0, Color::rgba(255, 255, 255, 90), valley_render::DEPTH_OVERLAY + 0.01);
            }
        }

        // NPC speech bubbles.
        let font = self.font.as_ref().unwrap();
        let white = rc.assets.white(rc.textures);
        {
            let mut ui = Ui { out, font, white, depth: valley_render::DEPTH_OVERLAY + 0.02 };
            for npc in w.npcs_here() {
                if let Some((text, _)) = &npc.bubble {
                    let p = cam.to_screen(npc.pos);
                    let m = ui.measure(text, 2.0);
                    let r = FRect::new(p.x + 32.0 - m.x / 2.0 - 8.0, p.y - 110.0, m.x + 16.0, 30.0);
                    ui.fill(r, Color::rgba(255, 255, 255, 230));
                    ui.text(text, Vec2::new(r.x + 8.0, r.y + 8.0), 2.0, ui::TEXT);
                }
            }
        }

        let mut ui = Ui { out, font, white, depth: DEPTH_HUD };
        hud::draw_hud(&mut rc, &mut ui, w, sw, sh, &self.toasts);

        ui.depth = DEPTH_HUD + 1.0;
        for m in &self.menus {
            draw_menu(&mut rc, &mut ui, w, m, sw, sh);
            ui.depth += 0.1;
        }
    }
}

// ---------------------------------------------------------------------- layout

fn title_buttons(sw: f32, sh: f32) -> Vec<FRect> {
    (0..3).map(|i| FRect::new(sw / 2.0 - 330.0 + i as f32 * 230.0, sh * 0.7, 200.0, 64.0)).collect()
}

fn load_row_rect(sw: f32, i: usize) -> FRect {
    FRect::new(sw / 2.0 - 360.0, 120.0 + i as f32 * 84.0, 720.0, 72.0)
}

fn create_rects(sw: f32, sh: f32) -> [FRect; 5] {
    let x = sw / 2.0 - 280.0;
    let y = sh / 2.0 - 150.0;
    [
        FRect::new(x, y, 560.0, 44.0),
        FRect::new(x, y + 90.0, 560.0, 44.0),
        FRect::new(x, y + 180.0, 560.0, 44.0),
        FRect::new(x, y + 250.0, 200.0, 56.0),
        FRect::new(x + 360.0, y + 250.0, 200.0, 56.0),
    ]
}

fn dialogue_box(sw: f32, sh: f32) -> FRect {
    let w = (sw - 80.0).min(1200.0);
    FRect::new((sw - w) / 2.0, sh - 300.0, w, 280.0)
}

fn question_option_rects(sw: f32, sh: f32, n: usize) -> Vec<FRect> {
    let b = dialogue_box(sw, sh);
    (0..n).map(|i| FRect::new(b.x + 32.0, b.y + 110.0 + i as f32 * 52.0, b.w - 64.0, 44.0)).collect()
}

fn menu_frame(sw: f32, sh: f32) -> FRect {
    let w = 12.0 * 68.0 + 96.0;
    let h = 560.0;
    FRect::new((sw - w) / 2.0, (sh - h) / 2.0, w, h)
}

fn inventory_tab_rects(sw: f32, sh: f32) -> Vec<FRect> {
    let f = menu_frame(sw, sh);
    (0..TABS.len()).map(|i| FRect::new(f.x + 16.0 + i as f32 * 166.0, f.y - 52.0, 156.0, 48.0)).collect()
}

fn known_recipes(w: &World) -> Vec<String> {
    w.farmer.crafting_recipes.keys().filter(|k| w.recipes.contains_key(*k)).cloned().collect()
}

fn recipe_rects(sw: f32, sh: f32) -> Vec<FRect> {
    let f = menu_frame(sw, sh);
    (0..40)
        .map(|i| {
            let row = i / 10;
            let col = i % 10;
            FRect::new(f.x + 64.0 + col as f32 * 84.0, f.y + 48.0 + row as f32 * 100.0, 72.0, 88.0)
        })
        .collect()
}

/// (player inventory rects, chest rects)
fn chest_rects(sw: f32, sh: f32) -> (Vec<FRect>, Vec<FRect>) {
    let f = menu_frame(sw, sh);
    let grid = |y0: f32| -> Vec<FRect> {
        (0..36)
            .map(|i| {
                let row = i / 12;
                let col = i % 12;
                FRect::new(f.x + 48.0 + col as f32 * 68.0, y0 + row as f32 * 68.0, 64.0, 64.0)
            })
            .collect()
    };
    (grid(f.y + 320.0), grid(f.y + 48.0))
}

fn inventory_slot_rects(sw: f32, sh: f32) -> Vec<FRect> {
    let f = menu_frame(sw, sh);
    (0..36)
        .map(|i| {
            let row = i / 12;
            let col = i % 12;
            FRect::new(f.x + 48.0 + col as f32 * 68.0, f.y + 48.0 + row as f32 * 72.0, 64.0, 64.0)
        })
        .collect()
}

fn trash_rect(sw: f32, sh: f32) -> Option<FRect> {
    let f = menu_frame(sw, sh);
    Some(FRect::new(f.x + f.w - 112.0, f.y + f.h - 112.0, 64.0, 64.0))
}

fn options_buttons(sw: f32, sh: f32) -> [FRect; 2] {
    let f = menu_frame(sw, sh);
    [FRect::new(f.x + 80.0, f.y + 120.0, 360.0, 64.0), FRect::new(f.x + 80.0, f.y + 210.0, 360.0, 64.0)]
}

fn shop_rows(sw: f32, sh: f32) -> Vec<FRect> {
    let f = menu_frame(sw, sh);
    (0..4).map(|i| FRect::new(f.x + 32.0, f.y + 40.0 + i as f32 * 84.0, f.w - 64.0, 76.0)).collect()
}

fn shop_inventory_rects(sw: f32, sh: f32) -> Vec<FRect> {
    let f = menu_frame(sw, sh);
    (0..36)
        .map(|i| {
            let row = i / 12;
            let col = i % 12;
            FRect::new(f.x + 48.0 + col as f32 * 68.0, f.y + 390.0 + row as f32 * 52.0, 48.0, 48.0)
        })
        .collect()
}

fn draw_menu(rc: &mut RenderCtx, ui: &mut Ui, w: &World, m: &Menu, sw: f32, sh: f32) {
    match m {
        Menu::Dialogue { npc, pages, page, shown } => {
            let b = dialogue_box(sw, sh);
            ui.panel(b);
            let p = &pages[*page];
            let text: String = p.text.chars().take(*shown as usize).collect();
            let has_portrait = npc.is_some();
            let text_w = if has_portrait { b.w - 380.0 } else { b.w - 64.0 };
            for (i, line) in ui.wrap(&text, text_w, 3.0).iter().enumerate() {
                ui.text(line, Vec2::new(b.x + 32.0, b.y + 32.0 + i as f32 * 34.0), 3.0, ui::TEXT);
            }
            if let Some(name) = npc {
                let npc_data = w.npcs.iter().find(|n| &n.name == name);
                let portrait = npc_data.map(|n| n.portrait.clone()).unwrap_or_else(|| format!("Portraits/{name}"));
                let display = npc_data.map(|n| n.display_name.clone()).unwrap_or_else(|| name.clone());
                let pr = FRect::new(b.x + b.w - 300.0, b.y + 24.0, 256.0, 256.0 - 48.0);
                ui.slot(FRect::new(pr.x - 4.0, pr.y - 4.0, 264.0, 192.0), false);
                if let Some(tex) = rc.assets.try_tex(rc.cm, rc.textures, &portrait) {
                    let idx = p.emotion.portrait_index();
                    let (tw, th) = rc.textures.size(tex);
                    let cols = (tw / 64).max(1) as i32;
                    let rows = (th / 64).max(1) as i32;
                    let idx = if idx >= cols * rows { 0 } else { idx };
                    ui.icon(tex, IRect::new((idx % cols) * 64, (idx / cols) * 64, 64, 64), FRect::new(pr.x + 36.0, pr.y, 184.0, 184.0));
                }
                ui.text_centered(&display, pr.x + 128.0, b.y + b.h - 56.0, 3.0, ui::TEXT);
            }
            if *page + 1 < pages.len() && *shown as usize >= p.text.chars().count() {
                ui.text("v", Vec2::new(b.x + b.w - 40.0, b.y + b.h - 40.0), 2.0, ui::TEXT);
            }
        }
        Menu::Message { text, shown } => {
            let b = dialogue_box(sw, sh);
            ui.panel(b);
            let t: String = text.chars().take(*shown as usize).collect();
            for (i, line) in ui.wrap(&t, b.w - 64.0, 3.0).iter().enumerate().take(7) {
                ui.text(line, Vec2::new(b.x + 32.0, b.y + 32.0 + i as f32 * 32.0), 3.0, ui::TEXT);
            }
        }
        Menu::Question { text, options, selected, .. } => {
            let b = dialogue_box(sw, sh);
            ui.panel(b);
            for (i, line) in ui.wrap(text, b.w - 64.0, 3.0).iter().enumerate().take(2) {
                ui.text(line, Vec2::new(b.x + 32.0, b.y + 32.0 + i as f32 * 32.0), 3.0, ui::TEXT);
            }
            for (i, (r, o)) in question_option_rects(sw, sh, options.len()).iter().zip(options).enumerate() {
                ui.slot(*r, i == *selected);
                ui.text(o, Vec2::new(r.x + 16.0, r.y + 12.0), 3.0, ui::TEXT);
            }
        }
        Menu::Shop { stock, scroll, hovered } => {
            let f = menu_frame(sw, sh);
            ui.panel(f);
            if let Some(owner) = &stock.owner {
                ui.text(owner, Vec2::new(f.x + 16.0, f.y - 30.0), 3.0, ui::TEXT_LIGHT);
            }
            for (row, r) in shop_rows(sw, sh).iter().enumerate() {
                let i = row + scroll;
                let Some(e) = stock.entries.get(i) else { break };
                ui.slot(*r, *hovered == Some(i));
                let (tex, src) = item_sprite(rc, w, &e.item);
                ui.icon(tex, src, FRect::new(r.x + 12.0, r.y + 6.0, 64.0, 64.0));
                ui.text(&w.items.display_name(&e.item), Vec2::new(r.x + 96.0, r.y + 26.0), 3.0, ui::TEXT);
                let price = format!("{}g", e.price);
                let pm = ui.measure(&price, 3.0);
                ui.text(&price, Vec2::new(r.x + r.w - pm.x - 24.0, r.y + 26.0), 3.0, if e.price > w.farmer.money { Color::RED } else { ui::TEXT });
            }
            ui.text(&format!("Money: {}g    (click: buy, shift: x5, click inventory: sell)", w.farmer.money), Vec2::new(f.x + 32.0, f.y + 360.0), 2.0, ui::TEXT);
            for (i, r) in shop_inventory_rects(sw, sh).iter().enumerate() {
                ui.slot(*r, false);
                if let Some(item) = w.farmer.inventory.get(i) {
                    hud::draw_item_in_slot(rc, ui, w, item, *r);
                }
            }
            if let Some(i) = hovered {
                if let Some(e) = stock.entries.get(*i) {
                    draw_tooltip(ui, w, &e.item, sw, sh);
                }
            }
        }
        Menu::Inventory { tab, held, hovered } => {
            let f = menu_frame(sw, sh);
            for (i, (r, label)) in inventory_tab_rects(sw, sh).iter().zip(TAB_LABELS).enumerate() {
                ui.button(*r, label, TABS[i] == *tab);
            }
            ui.panel(f);
            match tab {
                InvTab::Items => {
                    for (i, r) in inventory_slot_rects(sw, sh).iter().enumerate() {
                        let locked = i >= w.farmer.unlocked_slots;
                        ui.slot(*r, *held == Some(i) || (*held == None && i == w.farmer.current_slot));
                        if locked {
                            ui.fill(*r, Color::rgba(60, 40, 30, 140));
                            continue;
                        }
                        if let Some(item) = w.farmer.inventory.get(i) {
                            hud::draw_item_in_slot(rc, ui, w, item, *r);
                        }
                    }
                    if let Some(t) = trash_rect(sw, sh) {
                        ui.slot(t, false);
                        ui.text("Trash", Vec2::new(t.x + 4.0, t.y + 24.0), 2.0, ui::TEXT);
                    }
                    let info_y = f.y + 300.0;
                    ui.text(&format!("{}  -  {} Farm", w.farmer.name, w.farmer.farm_name), Vec2::new(f.x + 48.0, info_y), 3.0, ui::TEXT);
                    ui.text(&format!("Current funds: {}g", w.farmer.money), Vec2::new(f.x + 48.0, info_y + 44.0), 2.0, ui::TEXT);
                    ui.text(&format!("Total earnings: {}g", w.farmer.total_earned), Vec2::new(f.x + 48.0, info_y + 72.0), 2.0, ui::TEXT);
                    if let Some(i) = hovered {
                        if let Some(item) = w.farmer.inventory.get(*i) {
                            draw_tooltip(ui, w, item, sw, sh);
                        }
                    }
                }
                InvTab::Crafting => {
                    let names = known_recipes(w);
                    if names.is_empty() {
                        ui.text("You don't know any recipes yet.", Vec2::new(f.x + 64.0, f.y + 64.0), 2.0, ui::TEXT);
                    }
                    for (i, (r, name)) in recipe_rects(sw, sh).iter().zip(&names).enumerate() {
                        let recipe = &w.recipes[name];
                        let can = recipe.can_craft(&w.items, &w.farmer);
                        ui.slot(*r, *hovered == Some(i));
                        let out = recipe.output();
                        let (tex, src) = item_sprite(rc, w, &out);
                        if can {
                            ui.icon(tex, src, FRect::new(r.x + 4.0, r.y + 4.0, r.w - 8.0, r.h - 8.0));
                        } else {
                            ui.icon_tinted(tex, src, FRect::new(r.x + 4.0, r.y + 4.0, r.w - 8.0, r.h - 8.0), Color::rgba(90, 90, 90, 140));
                        }
                    }
                    if let Some(i) = hovered {
                        if let Some(name) = names.get(*i) {
                            draw_recipe_tooltip(rc, ui, w, &w.recipes[name], sw);
                        }
                    }
                }
                InvTab::Skills => {
                    use valley_sim::farmer::Skill;
                    for (i, s) in [Skill::Farming, Skill::Mining, Skill::Foraging, Skill::Fishing, Skill::Combat].iter().enumerate() {
                        let y = f.y + 48.0 + i as f32 * 80.0;
                        let lvl = w.farmer.skills.level(*s);
                        ui.text(&format!("{s:?}"), Vec2::new(f.x + 48.0, y + 16.0), 3.0, ui::TEXT);
                        for k in 0..10 {
                            let r = FRect::new(f.x + 320.0 + k as f32 * 52.0, y, 44.0, 44.0);
                            ui.slot(r, false);
                            if k < lvl {
                                ui.fill(FRect::new(r.x + 8.0, r.y + 8.0, r.w - 16.0, r.h - 16.0), Color::rgb(240, 180, 40));
                            }
                        }
                    }
                }
                InvTab::Social => {
                    let mut y = f.y + 48.0;
                    for npc in &w.npcs {
                        let pts = w.farmer.friendships.get(&npc.name).map(|f| f.points).unwrap_or(0);
                        let hearts = pts / 250;
                        ui.text(&npc.display_name, Vec2::new(f.x + 48.0, y + 12.0), 3.0, ui::TEXT);
                        for k in 0..10 {
                            let r = FRect::new(f.x + 320.0 + k as f32 * 40.0, y + 6.0, 30.0, 30.0);
                            ui.fill(r, if k < hearts { Color::rgb(230, 50, 80) } else { Color::rgb(200, 170, 130) });
                        }
                        let talked = w.farmer.friendships.get(&npc.name).is_some_and(|f| f.talked_today);
                        if talked {
                            ui.text("talked today", Vec2::new(f.x + 740.0, y + 14.0), 2.0, ui::TEXT);
                        }
                        y += 56.0;
                    }
                }
                InvTab::Options => {
                    let b = options_buttons(sw, sh);
                    ui.button(b[0], "Save & Exit to Title", false);
                    ui.button(b[1], "Quit Game", false);
                    ui.text("Controls: WASD move, C/left click use tool, X/right click act,", Vec2::new(f.x + 48.0, f.y + 320.0), 2.0, ui::TEXT);
                    ui.text("1-0,-,= toolbar, E/Esc menu, Shift walk, Ctrl+Tab switch game.", Vec2::new(f.x + 48.0, f.y + 348.0), 2.0, ui::TEXT);
                }
            }
        }
        Menu::Chest { tile, hovered } => {
            let f = menu_frame(sw, sh);
            ui.panel(f);
            let chest = match w.location().object(*tile).map(|o| &o.kind) {
                Some(valley_sim::objects::ObjKind::Chest(inv)) => Some(inv.clone()),
                _ => None,
            };
            let (prow, crow) = chest_rects(sw, sh);
            for (i, r) in crow.iter().enumerate() {
                ui.slot(*r, *hovered == Some((false, i)));
                if let Some(item) = chest.as_ref().and_then(|c| c.get(i)) {
                    hud::draw_item_in_slot(rc, ui, w, item, *r);
                }
            }
            ui.fill(FRect::new(f.x + 40.0, f.y + 270.0, f.w - 80.0, 4.0), ui::FRAME_LIGHT);
            for (i, r) in prow.iter().enumerate() {
                ui.slot(*r, *hovered == Some((true, i)));
                if let Some(item) = w.farmer.inventory.get(i) {
                    hud::draw_item_in_slot(rc, ui, w, item, *r);
                }
            }
            let hovered_item = match hovered {
                Some((true, i)) => w.farmer.inventory.get(*i).cloned(),
                Some((false, i)) => chest.as_ref().and_then(|c| c.get(*i).cloned()),
                None => None,
            };
            if let Some(item) = hovered_item {
                draw_tooltip(ui, w, &item, sw, sh);
            }
        }
        Menu::DayEnd { report, .. } => {
            ui.fill(FRect::new(0.0, 0.0, sw, sh), Color::rgb(30, 20, 40));
            let f = menu_frame(sw, sh);
            ui.panel(f);
            let title = if report.passed_out { "You passed out..." } else { "Good night!" };
            ui.text_centered(title, sw / 2.0, f.y + 32.0, 3.0, ui::TEXT);
            let mut y = f.y + 90.0;
            for (item, price) in report.shipped.iter().take(8) {
                let (tex, src) = item_sprite(rc, w, item);
                ui.icon(tex, src, FRect::new(f.x + 48.0, y, 40.0, 40.0));
                ui.text(&format!("{} x{}", w.items.display_name(item), item.stack()), Vec2::new(f.x + 100.0, y + 12.0), 2.0, ui::TEXT);
                let ps = format!("{price}g");
                let m = ui.measure(&ps, 2.0);
                ui.text(&ps, Vec2::new(f.x + f.w - m.x - 48.0, y + 12.0), 2.0, ui::TEXT);
                y += 44.0;
            }
            if report.shipped.is_empty() {
                ui.text_centered("Nothing was shipped today.", sw / 2.0, y, 2.0, ui::TEXT);
            }
            ui.text_centered(&format!("Total: {}g", report.earned), sw / 2.0, f.y + f.h - 120.0, 3.0, ui::TEXT);
            if report.money_lost > 0 {
                ui.text_centered(&format!("Lost {}g after passing out.", report.money_lost), sw / 2.0, f.y + f.h - 80.0, 2.0, Color::RED);
            }
            ui.text_centered(&format!("{} {}, Year {}", w.clock.season.name(), w.clock.day, w.clock.year), sw / 2.0, f.y + f.h - 48.0, 2.0, ui::TEXT);
        }
    }
}

fn draw_tooltip(ui: &mut Ui, w: &World, item: &valley_sim::item::Item, sw: f32, _sh: f32) {
    let name = w.items.display_name(item);
    let desc = w.items.description(item);
    let lines = ui.wrap(&desc, 360.0, 2.0);
    let h = 56.0 + lines.len() as f32 * 22.0 + 30.0;
    let x = (ui_mouse_hint().x + 24.0).min(sw - 420.0);
    let r = FRect::new(x, ui_mouse_hint().y + 24.0, 400.0, h);
    let d = ui.depth;
    ui.depth += 0.05;
    ui.panel(r);
    ui.text(&name, Vec2::new(r.x + 20.0, r.y + 20.0), 3.0, ui::TEXT);
    for (i, l) in lines.iter().enumerate() {
        ui.text(l, Vec2::new(r.x + 20.0, r.y + 56.0 + i as f32 * 22.0), 2.0, ui::TEXT);
    }
    let price = w.items.sell_price(item);
    if price > 0 {
        ui.text(&format!("Sells for {price}g"), Vec2::new(r.x + 20.0, r.y + h - 34.0), 2.0, ui::TEXT);
    }
    ui.depth = d;
}

fn draw_recipe_tooltip(rc: &mut RenderCtx, ui: &mut Ui, w: &World, r: &valley_sim::crafting::Recipe, sw: f32) {
    let out = r.output();
    let name = w.items.display_name(&out);
    let h = 70.0 + r.ingredients.len() as f32 * 44.0 + 20.0;
    let m = ui_mouse_hint();
    let rect = FRect::new((m.x + 24.0).min(sw - 440.0), m.y + 24.0, 420.0, h);
    let d = ui.depth;
    ui.depth += 0.05;
    ui.panel(rect);
    ui.text(&name, Vec2::new(rect.x + 20.0, rect.y + 20.0), 3.0, ui::TEXT);
    for (i, (ing, n)) in r.ingredients.iter().enumerate() {
        let y = rect.y + 60.0 + i as f32 * 44.0;
        let have: i32 = w.farmer.inventory.slots.iter().flatten().filter(|it| it.object_id() == Some(ing.as_str())).map(|it| it.stack()).sum();
        let label = match w.items.object(ing) {
            Some(o) => o.display_name.clone(),
            None => format!("Any ({ing})"),
        };
        let item = valley_sim::item::Item::object(ing, 1);
        let (tex, src) = item_sprite(rc, w, &item);
        ui.icon(tex, src, FRect::new(rect.x + 20.0, y, 36.0, 36.0));
        ui.text(&format!("{label} x{n}"), Vec2::new(rect.x + 68.0, y + 10.0), 2.0, if have >= *n { ui::TEXT } else { Color::RED });
    }
    ui.depth = d;
}

thread_local! {
    static MOUSE: std::cell::Cell<(f32, f32)> = const { std::cell::Cell::new((0.0, 0.0)) };
}

fn ui_mouse_hint() -> Vec2 {
    let (x, y) = MOUSE.with(|m| m.get());
    Vec2::new(x, y)
}

impl GameModule for ValleyGame {
    fn id(&self) -> &str {
        MODULE_ID
    }

    fn init(&mut self, ctx: &mut EngineCtx) {
        self.font = Some(BuiltinFont::install(&mut ctx.textures));
    }

    fn update(&mut self, ctx: &mut EngineCtx) {
        MOUSE.with(|m| m.set((ctx.input.mouse.x, ctx.input.mouse.y)));
        self.fade.alpha += (self.fade.target - self.fade.alpha).clamp(-0.05, 0.05);
        if self.fade.target >= 1.0 && self.fade.alpha >= 1.0 {
            self.fade.target = 0.0;
        }
        match self.screen {
            Screen::Title { .. } => self.update_title(ctx),
            Screen::LoadList { .. } => self.update_load_list(ctx),
            Screen::Create { .. } => self.update_create(ctx),
            Screen::Playing => self.update_playing(ctx),
        }
    }

    fn draw(&mut self, ctx: &mut EngineCtx, out: &mut DrawList) {
        match self.screen {
            Screen::Title { .. } => self.draw_title(ctx, out),
            Screen::LoadList { .. } => self.draw_load_list(ctx, out),
            Screen::Create { .. } => self.draw_create(ctx, out),
            Screen::Playing => self.draw_playing(ctx, out),
        }
        if self.fade.alpha > 0.0 {
            out.fill(FRect::new(0.0, 0.0, ctx.screen_w as f32, ctx.screen_h as f32), Color::BLACK.with_alpha(self.fade.alpha), 100.0);
        }
    }
}
