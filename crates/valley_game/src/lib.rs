//! The playable game as an engine [`GameModule`].

pub mod hud;
pub mod input_map;
pub mod menus;
pub mod music;
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
    Create { name: String, farm: String, favorite: String, male: bool, field: Field, look: valley_sim::farmer::Appearance },
    Playing,
}

/// Fade-to-black transition.
#[derive(Default)]
struct Fade {
    /// 0 = clear, 1 = black.
    alpha: f32,
    target: f32,
}

/// Building placement mode (free camera over the farm).
struct Placement {
    kind: String,
    cam: Vec2,
}

/// Player options, stored next to the saves.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Options {
    /// World zoom (1.0 = 4x pixel art).
    pub zoom: f32,
    pub music_volume: f32,
    pub sound_volume: f32,
    pub bindings: input_map::Bindings,
}

impl Default for Options {
    fn default() -> Self {
        Self { zoom: 1.0, music_volume: 0.75, sound_volume: 1.0, bindings: Default::default() }
    }
}

/// In-game console (F1).
#[derive(Default)]
struct Console {
    line: String,
    history: Vec<String>,
}

pub struct ValleyGame {
    /// Remaining ticks of a lightning flash.
    flash: u32,
    pub options: Options,
    console: Option<Console>,
    debug_overlay: bool,
    /// Binding row waiting for a key press.
    rebinding: Option<usize>,
    placement: Option<Placement>,
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
    /// Music track last requested from the engine ("" = silence).
    music: Option<String>,
}

impl ValleyGame {
    pub fn new(cm: ContentManager, save_dir: PathBuf) -> Self {
        let options = std::fs::read_to_string(save_dir.join("..").join("options.json")).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        let options: Options = options;
        input_map::set_bindings(&options.bindings);
        Self {
            flash: 0,
            options,
            console: None,
            debug_overlay: false,
            rebinding: None,
            placement: None,
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
            music: None,
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

    fn save_options(&self) {
        let p = self.save_dir.join("..").join("options.json");
        if let Some(d) = p.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let _ = std::fs::write(p, serde_json::to_string_pretty(&self.options).unwrap_or_default());
    }

    /// Ask the engine for the right music track when it changes.
    fn update_music(&mut self, ctx: &mut EngineCtx) {
        let want = match (&self.screen, &self.world) {
            (Screen::Playing, Some(w)) => music::track_for_world(w, self.music.as_deref()),
            (Screen::Playing, None) => None,
            _ => Some(music::TITLE_MUSIC.to_string()),
        };
        let Some(track) = want else { return };
        if self.music.as_deref() == Some(track.as_str()) {
            return;
        }
        if track.is_empty() {
            ctx.audio.stop_music();
        } else {
            ctx.audio.cmds.push(engine::SoundCmd::Music { track: track.clone(), volume: self.options.music_volume });
        }
        self.music = Some(track);
    }

    /// Camera in *world-view* pixels (the screen divided by the zoom).
    fn camera(&self, ctx: &EngineCtx) -> Option<Camera> {
        let w = self.world.as_ref()?;
        let loc = w.location();
        let z = self.options.zoom.max(0.25);
        Some(Camera::follow(
            w.event_camera_focus().unwrap_or_else(|| w.farmer.center()),
            loc.width() as f32 * TILE_F,
            loc.height() as f32 * TILE_F,
            ctx.screen_w as f32 / z,
            ctx.screen_h as f32 / z,
        ))
    }

    /// Mouse position in world-view pixels.
    fn view_mouse(&self, ctx: &EngineCtx) -> Vec2 {
        ctx.input.mouse * (1.0 / self.options.zoom.max(0.25))
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
                self.screen = Screen::Create { name: String::new(), farm: String::new(), favorite: String::new(), male: true, field: Field::Name, look: Default::default() };
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
        let Screen::Create { name, farm, favorite, male, field, look } = &mut self.screen else { return };
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
            use valley_sim::farmer::{HAIR_COLORS, PANTS_COLORS, SHIRT_COLORS, SKIN_TONES};
            let cycle = |cur: [u8; 3], list: &[[u8; 3]], d: i32| -> [u8; 3] {
                let i = list.iter().position(|c| *c == cur).unwrap_or(0) as i32;
                list[(i + d).rem_euclid(list.len() as i32) as usize]
            };
            for (row, (left, right)) in look_rects(sw, sh).iter().enumerate() {
                let d = if left.contains(m) { -1 } else if right.contains(m) { 1 } else { 0 };
                if d == 0 {
                    continue;
                }
                match row {
                    0 => look.skin = cycle(look.skin, SKIN_TONES, d),
                    1 => look.hair = cycle(look.hair, HAIR_COLORS, d),
                    2 => look.shirt = cycle(look.shirt, SHIRT_COLORS, d),
                    3 => look.pants = cycle(look.pants, PANTS_COLORS, d),
                    _ => look.long_hair = !look.long_hair,
                }
            }
        }
        let ok = ctx.input.pressed(Key::Enter) || (ctx.input.mouse_pressed(MouseButton::Left) && rects[4].contains(ctx.input.mouse));
        if ok && !name.trim().is_empty() {
            let seed = self.new_game_seed.unwrap_or_else(|| {
                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(1)
            });
            let opts = NewGame {
                appearance: look.clone(),
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

    fn update_placement(&mut self, ctx: &mut EngineCtx) {
        let Some(p) = self.placement.as_mut() else { return };
        let speed = 12.0;
        let i = &ctx.input;
        if i.held(Key::A) || i.held(Key::Left) {
            p.cam.x -= speed;
        }
        if i.held(Key::D) || i.held(Key::Right) {
            p.cam.x += speed;
        }
        if i.held(Key::W) || i.held(Key::Up) {
            p.cam.y -= speed;
        }
        if i.held(Key::S) || i.held(Key::Down) {
            p.cam.y += speed;
        }
        if i.pressed(Key::Escape) || i.mouse_pressed(MouseButton::Right) {
            self.placement = None;
            return;
        }
        if i.mouse_pressed(MouseButton::Left) {
            let world_pos = p.cam + ctx.input.mouse;
            let tile = TilePos::from_world(world_pos);
            let kind = p.kind.clone();
            if let Some(w) = self.world.as_mut() {
                let size = w.buildings_data.get(&kind).map(|d| (d.size.x.max(1), d.size.y.max(1))).unwrap_or((1, 1));
                let origin = TilePos::new(tile.x - size.0 / 2, tile.y - size.1 / 2);
                match w.place_building(&kind, origin) {
                    Ok(()) => {
                        self.placement = None;
                        self.toast("Construction started!");
                    }
                    Err(e) => self.toast(e),
                }
            }
        }
    }

    fn update_console(&mut self, ctx: &mut EngineCtx) -> bool {
        if ctx.input.pressed(Key::F1) {
            self.console = if self.console.is_some() { None } else { Some(Console::default()) };
            return true;
        }
        if ctx.input.pressed(Key::F3) {
            self.debug_overlay = !self.debug_overlay;
        }
        let Some(c) = self.console.as_mut() else { return false };
        for ch in ctx.input.text.chars() {
            if (ch.is_ascii_graphic() || ch == ' ') && c.line.len() < 80 {
                c.line.push(ch);
            }
        }
        if ctx.input.pressed(Key::Backspace) {
            c.line.pop();
        }
        if ctx.input.pressed(Key::Escape) {
            self.console = None;
            return true;
        }
        if ctx.input.pressed(Key::Enter) {
            let cmd = std::mem::take(&mut c.line);
            let out = self.run_command(&cmd);
            let c = self.console.as_mut().unwrap();
            c.history.push(format!("> {cmd}"));
            c.history.push(out);
            if c.history.len() > 40 {
                c.history.drain(..c.history.len() - 40);
            }
            self.handle_world_events(ctx);
        }
        // The console eats input; keep the world animating.
        if let Some(w) = self.world.as_mut() {
            w.paused = true;
            w.update(&Default::default());
        }
        true
    }

    fn update_playing(&mut self, ctx: &mut EngineCtx) {
        if self.update_console(ctx) {
            return;
        }
        if self.placement.is_some() {
            self.update_placement(ctx);
            if let Some(w) = self.world.as_mut() {
                w.paused = true;
                w.update(&Default::default());
            }
            self.handle_world_events(ctx);
            return;
        }
        let cam = self.camera(ctx);
        let vm = self.view_mouse(ctx);
        self.cursor_tile = cam.map(|c| TilePos::from_world(c.to_world(vm)));

        if !self.menus.is_empty() {
            self.update_menu(ctx);
        } else if self.world.as_ref().is_some_and(|w| w.festival_roaming()) {
            if ctx.input.pressed(Key::Escape) {
                self.menus.push(Menu::Question {
                    text: "Leave the festival? You'll head home for the night.".into(),
                    options: vec!["Leave".into(), "Stay".into()],
                    selected: 1,
                    kind: QuestionKind::FestivalLeave,
                });
            } else if let Some(w) = self.world.as_mut() {
                let input = input_map::player_input(&ctx.input, None);
                w.update(&input);
            }
        } else if self.world.as_ref().is_some_and(|w| w.in_event()) {
            if ctx.input.pressed(Key::Escape) {
                if let Some(w) = self.world.as_mut() {
                    w.skip_event();
                }
            }
            if let Some(w) = self.world.as_mut() {
                w.update(&Default::default());
            }
        } else if input_map::menu_pressed(&ctx.input) {
            self.menus.push(Menu::Inventory { tab: InvTab::Items, held: None, hovered: None });
        } else if input_map::journal_pressed(&ctx.input) {
            self.menus.push(Menu::Journal { selected: 0 });
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
                WorldEvent::Sound(cue) => {
                    if self.options.sound_volume > 0.0 {
                        ctx.audio.cmds.push(engine::SoundCmd::Play { cue, volume: self.options.sound_volume, pitch: 1.0 });
                    }
                }
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
                WorldEvent::OpenCooking => self.menus.push(Menu::Cooking { hovered: None }),
                WorldEvent::Music(track) => ctx.audio.cmds.push(engine::SoundCmd::Music { track, volume: self.options.music_volume }),
                WorldEvent::OpenBundles => self.menus.push(Menu::Bundles { area: 0, hovered: None }),
                WorldEvent::Lightning => self.flash = 12,
                WorldEvent::DialogueQuestion { npc, prompt, choices } => {
                    let display = self.world.as_ref().and_then(|w| w.npcs.iter().find(|n| n.name == npc).map(|n| n.display_name.clone())).unwrap_or(npc);
                    let q = Menu::Question { text: format!("{display}: {prompt}"), options: choices, selected: 0, kind: QuestionKind::DialogueAnswer };
                    // Show after the dialogue pages already on screen.
                    let at = if matches!(self.menus.last(), Some(Menu::Dialogue { .. })) { self.menus.len() - 1 } else { self.menus.len() };
                    self.menus.insert(at, q);
                }
                WorldEvent::FestivalHost { festival } => self.menus.push(Menu::Question {
                    text: format!("Are you ready to begin the main event of the {festival}?"),
                    options: vec!["Yes".into(), "Not yet".into()],
                    selected: 0,
                    kind: QuestionKind::FestivalMain,
                }),
                WorldEvent::OpenMuseum => self.menus.push(Menu::Museum { hovered: None }),
                WorldEvent::OpenBlacksmith => {
                    let collected = self.world.as_mut().is_some_and(|w| w.collect_tool_upgrade());
                    if collected {
                        self.toast("Picked up your upgraded tool!");
                    } else {
                        self.menus.push(Menu::Blacksmith { hovered: None });
                    }
                }
                WorldEvent::Fade { to_black } => self.fade = if to_black { Fade { alpha: 0.0, target: 1.0 } } else { Fade { alpha: 1.0, target: 0.0 } },
                WorldEvent::EventEnded => self.fade = Fade { alpha: 1.0, target: 0.0 },
                WorldEvent::OpenAnimalShop(list) => self.menus.push(Menu::AnimalShop { list, hovered: None }),
                WorldEvent::OpenCarpenter(list) => self.menus.push(Menu::Carpenter { list, hovered: None }),
                WorldEvent::OpenBillboard => self.menus.push(Menu::Billboard),
                WorldEvent::OpenOrderBoard => self.menus.push(Menu::OrderBoard),
                WorldEvent::Flash => self.fade = Fade { alpha: 1.0, target: 0.0 },
                WorldEvent::ShaftPrompt => self.menus.push(Menu::Question {
                    text: "It's a long way down. Jump in?".into(),
                    options: vec!["Jump".into(), "Stay".into()],
                    selected: 1,
                    kind: QuestionKind::Shaft,
                }),
                WorldEvent::TravelPrompt { location, tile, fare } => {
                    let name = self.world.as_ref().and_then(|w| w.locations.get(&location)).map(|l| l.display_name.clone()).filter(|n| !n.is_empty()).unwrap_or(location.clone());
                    self.menus.push(Menu::Question {
                        text: format!("Travel to {name} for {fare}g?"),
                        options: vec!["Go".into(), "Not now".into()],
                        selected: 0,
                        kind: QuestionKind::Travel { location, tile, fare },
                    });
                }
                WorldEvent::Letter { letter, .. } => self.menus.push(Menu::Letter { letter, shown: 0.0 }),
                WorldEvent::QuestCompleted(t) => self.toast(format!("Quest complete: {t}")),
                WorldEvent::NewQuest(t) => self.toast(format!("New quest: {t} (F: journal)")),
                WorldEvent::ElevatorMenu { deepest } => {
                    let mut levels = vec![1];
                    levels.extend((5..=deepest).step_by(5));
                    let mut options: Vec<String> = levels.iter().map(|l| format!("Floor {l}")).collect();
                    options.push("Cancel".into());
                    self.menus.push(Menu::Question { text: "Which floor?".into(), options, selected: 0, kind: QuestionKind::Elevator { levels } });
                }
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
                    if kind == QuestionKind::DialogueAnswer {
                        action = Some(Box::new(move |g: &mut ValleyGame, _| {
                            if let Some(w) = g.world.as_mut() {
                                w.answer_question(i);
                            }
                        }));
                    } else if let QuestionKind::Elevator { levels } = &kind {
                        if let Some(&level) = levels.get(i) {
                            action = Some(Box::new(move |g: &mut ValleyGame, _| {
                                if let Some(w) = g.world.as_mut() {
                                    w.enter_mine_level(level);
                                }
                            }));
                        }
                    } else if i == 0 {
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
                            QuestionKind::Elevator { .. } => {}
                            QuestionKind::FestivalMain => {
                                if let Some(w) = g.world.as_mut() {
                                    w.festival_begin_main();
                                }
                            }
                            QuestionKind::FestivalLeave => {
                                if let Some(w) = g.world.as_mut() {
                                    w.leave_festival();
                                }
                            }
                            QuestionKind::DialogueAnswer => {}
                            QuestionKind::Shaft => {
                                if let Some(w) = g.world.as_mut() {
                                    w.jump_down_shaft();
                                }
                            }
                            QuestionKind::Travel { location, tile, fare } => {
                                g.fade = Fade { alpha: 1.0, target: 0.0 };
                                if let Some(w) = g.world.as_mut() {
                                    w.travel(&location, tile, fare);
                                }
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
                        let shop_id = stock.id.clone();
                        let n = if ctx.input.held(Key::LeftShift) { 5 } else if ctx.input.held(Key::LeftCtrl) { 25 } else { 1 };
                        action = Some(Box::new(move |g: &mut ValleyGame, _| {
                            let Some(w) = g.world.as_mut() else { return };
                            let money = w.farmer.money;
                            if w.buy_from(&shop_id, &entry, n) {
                                // Limited stock shrinks in the open menu too.
                                if let Some(Menu::Shop { stock, .. }) = g.menus.last_mut() {
                                    let bought = (money - w.farmer.money) / entry.price.max(1);
                                    if let Some(e) = stock.entries.get_mut(i).filter(|e| e.stock > 0) {
                                        e.stock = (e.stock - bought.max(1)).max(0);
                                    }
                                    stock.entries.retain(|e| e.stock != 0);
                                }
                            } else if entry.price * n > money {
                                g.toast("Not enough money.");
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
                            } else if let Some(e) = equip_rects(sw, sh).iter().position(|r| r.contains(ctx.input.mouse)) {
                                if let Some(w) = self.world.as_mut() {
                                    match *held {
                                        Some(h) => {
                                            w.equip(h);
                                            *held = None;
                                        }
                                        None => {
                                            w.unequip(e);
                                        }
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
                        if let Some(row) = self.rebinding {
                            let keys = ctx.input.pressed_keys();
                            if let Some(&k) = keys.iter().find(|k| **k != Key::Escape) {
                                let o = &mut self.options;
                                let slot = o.bindings.get_mut(row);
                                if !slot.contains(&k) {
                                    slot.insert(0, k);
                                    slot.truncate(2);
                                }
                                input_map::set_bindings(&o.bindings);
                                self.rebinding = None;
                                self.save_options();
                                return;
                            }
                            if ctx.input.pressed(Key::Escape) {
                                self.rebinding = None;
                                return;
                            }
                        }
                        if ctx.input.mouse_pressed(MouseButton::Left) {
                            if let Some(row) = binding_rects(sw, sh).iter().position(|r| r.contains(ctx.input.mouse)) {
                                self.rebinding = Some(row);
                            }
                            let adj = option_adjust_rects(sw, sh);
                            for (row, (minus, plus)) in adj.iter().enumerate() {
                                let delta = if minus.contains(ctx.input.mouse) { -1.0 } else if plus.contains(ctx.input.mouse) { 1.0 } else { 0.0 };
                                if delta != 0.0 {
                                    action = Some(Box::new(move |g: &mut ValleyGame, _| {
                                        let o = &mut g.options;
                                        match row {
                                            0 => o.zoom = (o.zoom + delta * 0.25).clamp(0.5, 2.0),
                                            1 => o.music_volume = (o.music_volume + delta * 0.1).clamp(0.0, 1.0),
                                            _ => o.sound_volume = (o.sound_volume + delta * 0.1).clamp(0.0, 1.0),
                                        }
                                        g.save_options();
                                    }));
                                }
                            }
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
            Menu::Cooking { hovered } => {
                let names = self.world.as_ref().map(known_cooking).unwrap_or_default();
                let rects = recipe_rects(sw, sh);
                *hovered = rects.iter().position(|r| r.contains(ctx.input.mouse)).filter(|&i| i < names.len());
                if ctx.input.mouse_pressed(MouseButton::Left) {
                    if let Some(i) = *hovered {
                        let name = names[i].clone();
                        action = Some(Box::new(move |g: &mut ValleyGame, _| {
                            if let Some(w) = g.world.as_mut() {
                                w.cook(&name);
                            }
                        }));
                    }
                }
                if input_map::menu_pressed(&ctx.input) || ctx.input.mouse_pressed(MouseButton::Right) {
                    close = true;
                }
            }
            Menu::Letter { letter, shown } => {
                let len = letter.text.chars().count() as f32;
                *shown = (*shown + 4.0).min(len);
                if confirm || ctx.input.pressed(Key::Escape) {
                    if *shown < len {
                        *shown = len;
                    } else {
                        close = true;
                    }
                }
            }
            Menu::AnimalShop { list, hovered } => {
                let rows = shop_rows(sw, sh);
                *hovered = rows.iter().position(|r| r.contains(ctx.input.mouse)).filter(|&i| i < list.len());
                if ctx.input.mouse_pressed(MouseButton::Left) {
                    if let Some(i) = *hovered {
                        let kind = list[i].0.clone();
                        action = Some(Box::new(move |g: &mut ValleyGame, _| {
                            let r = g.world.as_mut().map(|w| w.buy_animal(&kind, None));
                            match r {
                                Some(Ok(())) => g.toast(format!("A new {kind} moved in!")),
                                Some(Err(e)) => g.toast(e),
                                None => {}
                            }
                        }));
                    }
                }
                if input_map::menu_pressed(&ctx.input) || ctx.input.mouse_pressed(MouseButton::Right) {
                    close = true;
                }
            }
            Menu::Carpenter { list, hovered } => {
                let rows = shop_rows(sw, sh);
                *hovered = rows.iter().position(|r| r.contains(ctx.input.mouse)).filter(|&i| i < list.len());
                if ctx.input.mouse_pressed(MouseButton::Left) {
                    if let Some(i) = (*hovered).filter(|&i| list[i] == valley_sim::world::HOUSE_UPGRADE_ENTRY) {
                        let _ = i;
                        close = true;
                        action = Some(Box::new(|g: &mut ValleyGame, _| {
                            match g.world.as_mut().map(|w| w.start_house_upgrade()) {
                                Some(Ok(())) => g.toast("Your farmhouse will be ready in 3 days."),
                                Some(Err(e)) => g.toast(e),
                                None => {}
                            }
                        }));
                    } else if let Some(i) = *hovered {
                        let kind = list[i].clone();
                        close = true;
                        action = Some(Box::new(move |g: &mut ValleyGame, ctx: &mut EngineCtx| {
                            let Some(w) = g.world.as_ref() else { return };
                            let farm = &w.locations["Farm"];
                            let center = Vec2::new(farm.width() as f32 * TILE_F / 2.0, farm.height() as f32 * TILE_F / 2.0);
                            let cam = center - Vec2::new(ctx.screen_w as f32 / 2.0, ctx.screen_h as f32 / 2.0);
                            g.placement = Some(Placement { kind, cam });
                        }));
                    }
                }
                if input_map::menu_pressed(&ctx.input) || ctx.input.mouse_pressed(MouseButton::Right) {
                    close = true;
                }
            }
            Menu::OrderBoard => {
                let offers = self.world.as_ref().map(|w| w.order_offers.clone()).unwrap_or_default();
                let busy = self.world.as_ref().is_some_and(|w| w.farmer.special_order.is_some());
                let clicked = order_accept_rects(sw, sh).iter().position(|r| r.contains(ctx.input.mouse)).filter(|_| ctx.input.mouse_pressed(MouseButton::Left));
                if let Some(id) = clicked.and_then(|i| offers.get(i)).filter(|_| !busy).cloned() {
                    close = true;
                    action = Some(Box::new(move |g: &mut ValleyGame, _| {
                        if let Some(Err(e)) = g.world.as_mut().map(|w| w.accept_order(&id)) {
                            g.toast(&e);
                        }
                    }));
                } else if input_map::menu_pressed(&ctx.input) || ctx.input.mouse_pressed(MouseButton::Right) || ctx.input.pressed(Key::Enter) {
                    close = true;
                }
            }
            Menu::Billboard => {
                let accept = billboard_accept_rect(sw, sh);
                if ctx.input.mouse_pressed(MouseButton::Left) && accept.contains(ctx.input.mouse) {
                    action = Some(Box::new(|g: &mut ValleyGame, _| {
                        if let Some(w) = g.world.as_mut() {
                            w.accept_daily_quest();
                        }
                    }));
                } else if input_map::menu_pressed(&ctx.input) || ctx.input.mouse_pressed(MouseButton::Right) || ctx.input.pressed(Key::Enter) {
                    close = true;
                }
            }
            Menu::Journal { selected } => {
                let n = self.world.as_ref().map(|w| w.farmer.quests.len()).unwrap_or(0);
                if n > 0 {
                    if ctx.input.pressed(Key::Down) || ctx.input.pressed(Key::S) {
                        *selected = (*selected + 1) % n;
                    }
                    if ctx.input.pressed(Key::Up) || ctx.input.pressed(Key::W) {
                        *selected = (*selected + n - 1) % n;
                    }
                    for (i, r) in journal_rows(sw, sh).iter().enumerate().take(n) {
                        if r.contains(ctx.input.mouse) && ctx.input.mouse_pressed(MouseButton::Left) {
                            *selected = i;
                        }
                    }
                }
                if input_map::menu_pressed(&ctx.input) || input_map::journal_pressed(&ctx.input) || ctx.input.mouse_pressed(MouseButton::Right) {
                    close = true;
                }
            }
            Menu::DayEnd { ms, .. } => {
                *ms += 16;
                if (*ms > 600 && confirm) || *ms > 15000 {
                    close = true;
                    action = Some(Box::new(|g: &mut ValleyGame, _| {
                        g.fade = Fade { alpha: 1.0, target: 0.0 };
                        let pending = g.world.as_mut().map(|w| std::mem::take(&mut w.farmer.pending_levelups)).unwrap_or_default();
                        // Show the highest level per skill, newest last.
                        for (skill, level) in pending.into_iter().rev() {
                            if !g.menus.iter().any(|m| matches!(m, Menu::LevelUp { skill: s, .. } if *s == skill)) {
                                g.menus.insert(0, Menu::LevelUp { skill, level, hovered: None });
                            }
                        }
                    }));
                }
            }
            Menu::Bundles { area, hovered } => {
                let areas = self.world.as_ref().map(|w| w.bundle_areas()).unwrap_or_default();
                let tabs = bundle_tab_rects(sw, sh, areas.len());
                if ctx.input.mouse_pressed(MouseButton::Left) {
                    if let Some(i) = tabs.iter().position(|r| r.contains(ctx.input.mouse)) {
                        *area = i;
                    }
                }
                let keys: Vec<(String, usize)> = self
                    .world
                    .as_ref()
                    .map(|w| w.bundles.iter().filter(|b| areas.get(*area) == Some(&b.area)).map(|b| (b.key.clone(), b.reqs.len())).collect())
                    .unwrap_or_default();
                *hovered = None;
                for (bi, (_, n)) in keys.iter().enumerate() {
                    for ri in 0..*n {
                        if bundle_req_rect(sw, sh, bi, ri).contains(ctx.input.mouse) {
                            *hovered = Some((bi, ri));
                        }
                    }
                }
                if ctx.input.mouse_pressed(MouseButton::Left) {
                    if let Some((bi, ri)) = *hovered {
                        let key = keys[bi].0.clone();
                        action = Some(Box::new(move |g: &mut ValleyGame, _| {
                            if let Some(Err(e)) = g.world.as_mut().map(|w| w.donate_to_bundle(&key, ri)) {
                                g.toast(e);
                            }
                        }));
                    }
                }
                if input_map::menu_pressed(&ctx.input) || ctx.input.mouse_pressed(MouseButton::Right) {
                    close = true;
                }
            }
            Menu::Museum { hovered } => {
                let slots = shop_inventory_rects(sw, sh);
                *hovered = slots.iter().position(|r| r.contains(ctx.input.mouse));
                if ctx.input.mouse_pressed(MouseButton::Left) {
                    if let Some(i) = *hovered {
                        action = Some(Box::new(move |g: &mut ValleyGame, _| {
                            match g.world.as_mut().map(|w| w.donate_to_museum(i)) {
                                Some(Ok(())) => g.toast("Thank you for your donation!"),
                                Some(Err(e)) => g.toast(e),
                                None => {}
                            }
                        }));
                    }
                }
                if input_map::menu_pressed(&ctx.input) || ctx.input.mouse_pressed(MouseButton::Right) {
                    close = true;
                }
            }
            Menu::Blacksmith { hovered } => {
                let rows = shop_rows(sw, sh);
                let tools = self.world.as_ref().map(upgradable_tools).unwrap_or_default();
                *hovered = rows.iter().position(|r| r.contains(ctx.input.mouse)).filter(|&i| i < tools.len());
                let geode = blacksmith_geode_rect(sw, sh);
                if ctx.input.mouse_pressed(MouseButton::Left) {
                    if let Some(i) = *hovered {
                        let slot = tools[i];
                        close = true;
                        action = Some(Box::new(move |g: &mut ValleyGame, _| {
                            let r = g.world.as_mut().map(|w| w.start_tool_upgrade(slot));
                            match r {
                                Some(Ok(())) => g.toast("Come back in 2 days."),
                                Some(Err(e)) => g.toast(e),
                                None => {}
                            }
                        }));
                    } else if geode.contains(ctx.input.mouse) {
                        action = Some(Box::new(|g: &mut ValleyGame, _| {
                            let slot = g.world.as_ref().map(|w| w.farmer.current_slot).unwrap_or(0);
                            let r = g.world.as_mut().map(|w| w.crack_geode(slot));
                            if let Some(Err(e)) = r {
                                g.toast(e);
                            }
                        }));
                    }
                }
                if input_map::menu_pressed(&ctx.input) || ctx.input.mouse_pressed(MouseButton::Right) {
                    close = true;
                }
            }
            Menu::LevelUp { skill, level, hovered } => {
                let owned = self.world.as_ref().map(|w| w.farmer.professions.clone()).unwrap_or_default();
                let choices = if *level == 5 || *level == 10 { valley_sim::professions::choices(*skill, *level, &owned) } else { vec![] };
                let rects = levelup_choice_rects(sw, sh);
                *hovered = rects.iter().position(|r| r.contains(ctx.input.mouse)).filter(|&i| i < choices.len());
                if choices.is_empty() {
                    if confirm {
                        close = true;
                    }
                } else if ctx.input.mouse_pressed(MouseButton::Left) {
                    if let Some(i) = *hovered {
                        let id = choices[i].id;
                        close = true;
                        action = Some(Box::new(move |g: &mut ValleyGame, _| {
                            if let Some(w) = g.world.as_mut() {
                                w.choose_profession(id);
                            }
                        }));
                    }
                }
            }
        }
        if close {
            self.menus.pop();
            if self.menus.is_empty() {
                if let Some(w) = self.world.as_mut() {
                    w.event_resume();
                }
            }
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
        let Screen::Create { name, farm, favorite, male, field, look } = &self.screen else { return };
        let look = look.clone();
        let mut rc = RenderCtx { cm: &mut self.cm, textures: &mut ctx.textures, assets: &mut self.assets };
        let farmer_tex = valley_render::world_draw::farmer_texture(&mut rc, &look);
        let font = self.font.as_ref().unwrap();
        let white = self.assets.white(&mut ctx.textures);
        let mut ui = Ui { out, font, white, depth: DEPTH_HUD };
        let cx = create_cx(sw);
        ui.panel(FRect::new(cx - 360.0, sh / 2.0 - 260.0, 720.0, 520.0));
        let lp = look_panel(sw, sh);
        ui.panel(lp);
        let preview_frame = ((ctx.tick / 12) % 4) as i32;
        let dir_row = ((ctx.tick / 120) % 4) as i32;
        ui.slot(FRect::new(lp.x + lp.w / 2.0 - 70.0, lp.y + 30.0, 140.0, 180.0), false);
        ui.icon(farmer_tex, IRect::new(preview_frame * 16, dir_row * 32, 16, 32), FRect::new(lp.x + lp.w / 2.0 - 40.0, lp.y + 40.0, 80.0, 160.0));
        for (i, ((l, r), label)) in look_rects(sw, sh).iter().zip(["Skin", "Hair", "Shirt", "Pants", "Style"]).enumerate() {
            let _ = i;
            ui.button(*l, "<", false);
            ui.button(*r, ">", false);
            ui.text_centered(label, (l.x + l.w + r.x) / 2.0, l.y + 14.0, 2.0, ui::TEXT);
        }
        ui.text_centered("New Farmer", cx, sh / 2.0 - 230.0, 3.0, ui::TEXT);
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
        ui.text_centered("Tab: next field   Enter: start   Esc: back", cx, sh / 2.0 + 220.0, 2.0, ui::TEXT);
    }

    fn draw_placement(&mut self, ctx: &mut EngineCtx, out: &mut DrawList) {
        let (sw, sh) = (ctx.screen_w as f32, ctx.screen_h as f32);
        out.clear(Color::BLACK);
        let (Some(p), Some(w)) = (self.placement.as_ref(), self.world.as_ref()) else { return };
        let cam = Camera { pos: p.cam, screen_w: sw, screen_h: sh };
        let mut rc = RenderCtx { cm: &mut self.cm, textures: &mut ctx.textures, assets: &mut self.assets };
        valley_render::world_draw::draw_location_view(&mut rc, out, w, "Farm", &cam);
        let tile = TilePos::from_world(p.cam + ctx.input.mouse);
        let size = w.buildings_data.get(&p.kind).map(|d| (d.size.x.max(1), d.size.y.max(1))).unwrap_or((1, 1));
        let origin = TilePos::new(tile.x - size.0 / 2, tile.y - size.1 / 2);
        let farm = &w.locations["Farm"];
        for dy in 0..size.1 {
            for dx in 0..size.0 {
                let t = TilePos::new(origin.x + dx, origin.y + dy);
                let ok = farm.is_clear(t) && valley_sim::collision::tile_passable(farm, t) && !(w.current == "Farm" && w.farmer.tile() == t);
                let sp = cam.to_screen(Vec2::new(t.x as f32 * TILE_F, t.y as f32 * TILE_F));
                let c = if ok { Color::rgba(80, 220, 80, 110) } else { Color::rgba(230, 60, 60, 130) };
                out.fill(FRect::new(sp.x + 2.0, sp.y + 2.0, TILE_F - 4.0, TILE_F - 4.0), c, valley_render::DEPTH_OVERLAY + 0.01);
            }
        }
        let font = self.font.as_ref().unwrap();
        let white = rc.assets.white(rc.textures);
        let mut ui = Ui { out, font, white, depth: DEPTH_HUD };
        let name = w.buildings_data.get(&p.kind).map(|d| d.name.clone()).unwrap_or_default();
        let r = FRect::new(16.0, 16.0, 720.0, 72.0);
        ui.panel(r);
        ui.text(&format!("Place: {name}   (WASD: move view, click: build, Esc: cancel)"), Vec2::new(r.x + 20.0, r.y + 28.0), 2.0, ui::TEXT);
    }

    fn draw_playing(&mut self, ctx: &mut EngineCtx, out: &mut DrawList) {
        if self.placement.is_some() {
            self.draw_placement(ctx, out);
            return;
        }
        let (sw, sh) = (ctx.screen_w as f32, ctx.screen_h as f32);
        out.clear(Color::BLACK);
        let Some(cam) = self.camera(ctx) else { return };
        let Some(w) = self.world.as_ref() else { return };
        let mut rc = RenderCtx { cm: &mut self.cm, textures: &mut ctx.textures, assets: &mut self.assets };
        let first_world_cmd = out.cmds.len();
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
        // Apply world zoom to everything drawn so far (the HUD is drawn after).
        let z = self.options.zoom;
        if (z - 1.0).abs() > 1e-3 {
            for cmd in out.cmds[first_world_cmd..].iter_mut() {
                match cmd {
                    engine::DrawCmd::Sprite(s) => {
                        // Full-screen overlays stay full-screen.
                        s.dst = FRect::new(s.dst.x * z, s.dst.y * z, s.dst.w * z, s.dst.h * z);
                        s.origin = s.origin * z;
                    }
                    engine::DrawCmd::Fill { rect, .. } => {
                        *rect = FRect::new(rect.x * z, rect.y * z, rect.w * z, rect.h * z);
                    }
                }
            }
        }

        let mut ui = Ui { out, font, white, depth: DEPTH_HUD };
        hud::draw_hud(&mut rc, &mut ui, w, sw, sh, &self.toasts);
        ui.depth = DEPTH_HUD + 0.5;
        hud::draw_fishing_overlay(&mut rc, &mut ui, w, sw, sh);

        ui.depth = DEPTH_HUD + 1.0;
        for m in &self.menus {
            draw_menu(&mut rc, &mut ui, w, m, sw, sh);
            ui.depth += 0.1;
        }
        if self.debug_overlay {
            ui.depth = DEPTH_HUD + 5.0;
            let z = self.options.zoom;
            let loc = w.location();
            let (x0, y0, x1, y1) = cam.tile_range(TILE_F, 0);
            for y in y0.max(0)..=y1.min(loc.height() - 1) {
                for x in x0.max(0)..=x1.min(loc.width() - 1) {
                    let t = TilePos::new(x, y);
                    let p = cam.to_screen(Vec2::new(x as f32 * TILE_F, y as f32 * TILE_F)) * z;
                    let r = FRect::new(p.x + 1.0, p.y + 1.0, TILE_F * z - 2.0, TILE_F * z - 2.0);
                    if !valley_sim::collision::tile_passable(loc, t) {
                        ui.out.fill(r, Color::rgba(220, 40, 40, 60), ui.depth);
                    }
                    if loc.action_at(t).is_some() {
                        ui.out.outline(r, 3.0, Color::rgba(250, 220, 60, 200), ui.depth + 0.001);
                    }
                }
            }
            for wp in &loc.warps {
                let p = cam.to_screen(Vec2::new(wp.from.x as f32 * TILE_F, wp.from.y as f32 * TILE_F)) * z;
                ui.out.fill(FRect::new(p.x, p.y, TILE_F * z, TILE_F * z), Color::rgba(60, 120, 250, 110), ui.depth + 0.001);
            }
            let ft = w.farmer.tile();
            let tt = w.target_tile(self.cursor_tile);
            let info = format!(
                "{}  tile {},{}  target {},{}  {}  t{}  npcs {}  monsters {}  zoom {:.2}",
                w.current, ft.x, ft.y, tt.x, tt.y, w.time_string(), w.clock.time, w.npcs_here().count(), w.monsters.iter().filter(|m| m.location == w.current).count(), z
            );
            let m = ui.measure(&info, 2.0);
            ui.out.fill(FRect::new(8.0, sh - 220.0, m.x + 16.0, 30.0), Color::rgba(0, 0, 0, 170), ui.depth + 0.002);
            ui.text(&info, Vec2::new(16.0, sh - 212.0), 2.0, ui::TEXT_LIGHT);
        }
        if let Some(c) = &self.console {
            ui.depth = DEPTH_HUD + 6.0;
            let lines = 12usize;
            let h = 40.0 + lines as f32 * 22.0;
            ui.out.fill(FRect::new(0.0, 0.0, sw, h), Color::rgba(10, 10, 20, 220), ui.depth);
            let start = c.history.len().saturating_sub(lines);
            for (i, l) in c.history[start..].iter().enumerate() {
                ui.text(l, Vec2::new(12.0, 8.0 + i as f32 * 22.0), 2.0, Color::rgb(200, 230, 200));
            }
            let caret = if (ctx.tick / 30) % 2 == 0 { "_" } else { " " };
            ui.text(&format!("] {}{caret}", c.line), Vec2::new(12.0, h - 28.0), 2.0, Color::rgb(255, 255, 160));
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

/// Center of the name/farm form (shifted left to make room for the look panel).
fn create_cx(sw: f32) -> f32 {
    sw / 2.0 - 190.0
}

fn create_rects(sw: f32, sh: f32) -> [FRect; 5] {
    let x = create_cx(sw) - 280.0;
    let y = sh / 2.0 - 150.0;
    [
        FRect::new(x, y, 560.0, 44.0),
        FRect::new(x, y + 90.0, 560.0, 44.0),
        FRect::new(x, y + 180.0, 560.0, 44.0),
        FRect::new(x, y + 250.0, 200.0, 56.0),
        FRect::new(x + 360.0, y + 250.0, 200.0, 56.0),
    ]
}

/// (left, right) arrow buttons for skin, hair, shirt, pants, hair style.
fn look_panel(sw: f32, sh: f32) -> FRect {
    FRect::new(create_cx(sw) + 380.0, sh / 2.0 - 260.0, 360.0, 520.0)
}

fn look_rects(sw: f32, sh: f32) -> Vec<(FRect, FRect)> {
    let p = look_panel(sw, sh);
    (0..5).map(|i| {
        let y = p.y + 230.0 + i as f32 * 54.0;
        (FRect::new(p.x + 30.0, y, 44.0, 44.0), FRect::new(p.x + p.w - 74.0, y, 44.0, 44.0))
    }).collect()
}

fn dialogue_box(sw: f32, sh: f32) -> FRect {
    let w = (sw - 80.0).min(1200.0);
    FRect::new((sw - w) / 2.0, sh - 300.0, w, 280.0)
}

fn question_option_rects(sw: f32, sh: f32, n: usize) -> Vec<FRect> {
    let b = question_box(sw, sh, n);
    let cols = if n > 4 { 3 } else { 1 };
    let w = (b.w - 64.0 - (cols - 1) as f32 * 12.0) / cols as f32;
    (0..n).map(|i| FRect::new(b.x + 32.0 + (i % cols) as f32 * (w + 12.0), b.y + 110.0 + (i / cols) as f32 * 52.0, w, 44.0)).collect()
}

fn question_box(sw: f32, sh: f32, n: usize) -> FRect {
    let b = dialogue_box(sw, sh);
    let cols = if n > 4 { 3 } else { 1 };
    let rows = n.div_ceil(cols) as f32;
    let h = (130.0 + rows * 52.0).max(b.h);
    FRect::new(b.x, (sh - h - 20.0).max(10.0), b.w, h)
}

/// Accept buttons under the two special order cards.
fn order_accept_rects(sw: f32, sh: f32) -> [FRect; 2] {
    let f = menu_frame(sw, sh);
    let half = (f.w - 96.0) / 2.0;
    let b = |i: usize| FRect::new(f.x + 32.0 + i as f32 * (half + 32.0) + (half - 200.0) / 2.0, f.y + f.h - 96.0, 200.0, 52.0);
    [b(0), b(1)]
}

fn menu_frame(sw: f32, sh: f32) -> FRect {
    let w = 12.0 * 68.0 + 96.0;
    let h = 560.0;
    FRect::new((sw - w) / 2.0, (sh - h) / 2.0, w, h)
}

fn inventory_tab_rects(sw: f32, sh: f32) -> Vec<FRect> {
    let f = menu_frame(sw, sh);
    (0..TABS.len()).map(|i| FRect::new(f.x + 8.0 + i as f32 * 148.0, f.y - 52.0, 140.0, 48.0)).collect()
}

fn bundle_tab_rects(sw: f32, sh: f32, n: usize) -> Vec<FRect> {
    let f = menu_frame(sw, sh);
    (0..n).map(|i| FRect::new(f.x + 16.0 + i as f32 * 200.0, f.y - 52.0, 190.0, 48.0)).collect()
}

fn bundle_req_rect(sw: f32, sh: f32, bundle: usize, req: usize) -> FRect {
    let f = menu_frame(sw, sh);
    FRect::new(f.x + 300.0 + req as f32 * 60.0, f.y + 40.0 + bundle as f32 * 84.0, 52.0, 52.0)
}

/// Inventory slots holding upgradable tools.
fn upgradable_tools(w: &World) -> Vec<usize> {
    use valley_sim::item::ToolKind;
    (0..w.farmer.inventory.capacity())
        .filter(|&i| {
            matches!(w.farmer.inventory.get(i), Some(valley_sim::item::Item::Tool(t)) if matches!(t.kind, ToolKind::Hoe | ToolKind::WateringCan | ToolKind::Axe | ToolKind::Pickaxe) && t.level < 4)
        })
        .take(4)
        .collect()
}

fn blacksmith_geode_rect(sw: f32, sh: f32) -> FRect {
    let f = menu_frame(sw, sh);
    FRect::new(f.x + 32.0, f.y + f.h - 96.0, 320.0, 60.0)
}

fn levelup_choice_rects(sw: f32, sh: f32) -> Vec<FRect> {
    let f = menu_frame(sw, sh);
    (0..2).map(|i| FRect::new(f.x + 40.0 + i as f32 * (f.w / 2.0 - 20.0), f.y + 160.0, f.w / 2.0 - 60.0, 260.0)).collect()
}

fn known_cooking(w: &World) -> Vec<String> {
    w.farmer.cooking_recipes.keys().filter(|k| w.cooking.contains_key(*k)).cloned().collect()
}

fn billboard_accept_rect(sw: f32, sh: f32) -> FRect {
    let f = menu_frame(sw, sh);
    FRect::new(f.x + f.w - 260.0, f.y + f.h - 90.0, 220.0, 60.0)
}

fn journal_rows(sw: f32, sh: f32) -> Vec<FRect> {
    let f = menu_frame(sw, sh);
    (0..8).map(|i| FRect::new(f.x + 32.0, f.y + 40.0 + i as f32 * 60.0, 360.0, 52.0)).collect()
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

/// Equipment slots in the items tab: ring 1, ring 2, boots (top row) and
/// hat, shirt, pants (second row); indices match `World::unequip`.
fn equip_rects(sw: f32, sh: f32) -> [FRect; 6] {
    let f = menu_frame(sw, sh);
    let x = f.x + f.w - 300.0;
    let y = f.y + 300.0;
    let r = |i: usize| FRect::new(x + (i % 3) as f32 * 76.0, y + (i / 3) as f32 * 76.0, 64.0, 64.0);
    [r(0), r(1), r(2), r(3), r(4), r(5)]
}

fn trash_rect(sw: f32, sh: f32) -> Option<FRect> {
    let f = menu_frame(sw, sh);
    Some(FRect::new(f.x + f.w - 112.0, f.y + f.h - 112.0, 64.0, 64.0))
}

fn options_buttons(sw: f32, sh: f32) -> [FRect; 2] {
    let f = menu_frame(sw, sh);
    [FRect::new(f.x + 80.0, f.y + 40.0, 360.0, 64.0), FRect::new(f.x + 80.0, f.y + 120.0, 360.0, 64.0)]
}

/// Rows of the key binding list in the options tab.
fn binding_rects(sw: f32, sh: f32) -> Vec<FRect> {
    let f = menu_frame(sw, sh);
    (0..input_map::BINDING_NAMES.len()).map(|i| FRect::new(f.x + 48.0 + (i / 5) as f32 * 430.0, f.y + 260.0 + (i % 5) as f32 * 52.0, 410.0, 44.0)).collect()
}

/// (minus, plus) buttons for zoom, music and sound rows.
fn option_adjust_rects(sw: f32, sh: f32) -> [(FRect, FRect); 3] {
    let f = menu_frame(sw, sh);
    let row = |i: f32| (FRect::new(f.x + f.w - 260.0, f.y + 40.0 + i * 70.0, 64.0, 56.0), FRect::new(f.x + f.w - 100.0, f.y + 40.0 + i * 70.0, 64.0, 56.0));
    [row(0.0), row(1.0), row(2.0)]
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
            let b = question_box(sw, sh, options.len());
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
                    let er = equip_rects(sw, sh);
                    for (k, (r, label)) in er.iter().zip(["Ring", "Ring", "Boots", "Hat", "Shirt", "Pants"]).enumerate() {
                        ui.slot(*r, false);
                        let item = match k {
                            0 | 1 => w.farmer.rings[k].as_ref(),
                            2 => w.farmer.boots.as_ref(),
                            3 => w.farmer.hat.as_ref(),
                            4 => w.farmer.shirt_item.as_ref(),
                            _ => w.farmer.pants_item.as_ref(),
                        };
                        match item {
                            Some(it) => hud::draw_item_in_slot(rc, ui, w, it, *r),
                            None => ui.text(label, Vec2::new(r.x + 4.0, r.y + 24.0), 1.5, Color::rgba(70, 36, 20, 140)),
                        }
                    }
                    let a = w.equipment_attrs();
                    if a.defense != 0 || a.attack != 0 || a.magnetic_radius != 0 {
                        ui.text(&a.describe().join(", "), Vec2::new(er[0].x, er[3].y + 76.0), 2.0, ui::TEXT);
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
                InvTab::Collections => {
                    // Everything sellable, colored once shipped or caught.
                    let ids: Vec<&String> = w.items.objects.iter().filter(|(_, o)| o.data.price > 0 && o.data.kind != "Litter").map(|(k, _)| k).collect();
                    let cols = 14;
                    for (n, id) in ids.iter().enumerate().take(cols * 4) {
                        let r = FRect::new(f.x + 40.0 + (n % cols) as f32 * 62.0, f.y + 40.0 + (n / cols) as f32 * 62.0, 56.0, 56.0);
                        ui.slot(r, false);
                        let got = w.farmer.shipped.contains_key(*id) || w.farmer.fish_caught.contains_key(*id) || w.farmer.bundles.museum.contains(id);
                        let item = valley_sim::item::Item::object(id, 1);
                        let (tex, src) = item_sprite(rc, w, &item);
                        let inner = FRect::new(r.x + 4.0, r.y + 4.0, r.w - 8.0, r.h - 8.0);
                        if got {
                            ui.icon(tex, src, inner);
                        } else {
                            ui.icon_tinted(tex, src, inner, Color::rgba(40, 30, 25, 160));
                        }
                    }
                    // Achievements: three columns, earned ones in gold.
                    let earned = w.farmer.achievements.len();
                    ui.text(&format!("Achievements ({earned}/{})", valley_sim::pets::ACHIEVEMENTS.len()), Vec2::new(f.x + 40.0, f.y + 300.0), 2.0, ui::TEXT);
                    for (i, a) in valley_sim::pets::ACHIEVEMENTS.iter().enumerate() {
                        let x = f.x + 40.0 + (i % 3) as f32 * 290.0;
                        let y = f.y + 326.0 + (i / 3) as f32 * 32.0;
                        let got = w.farmer.achievements.iter().any(|g| g == a.id);
                        ui.fill(FRect::new(x, y, 22.0, 22.0), if got { Color::rgb(240, 180, 40) } else { Color::rgb(190, 165, 130) });
                        let label = if got { a.name.to_string() } else { "???".to_string() };
                        ui.text(&label, Vec2::new(x + 30.0, y + 4.0), 2.0, if got { ui::TEXT } else { Color::rgb(150, 120, 90) });
                    }
                    let shipped: i32 = w.farmer.shipped.values().sum();
                    let fish: i32 = w.farmer.fish_caught.values().map(|v| v.0).sum();
                    let cooked: i32 = w.farmer.cooking_recipes.values().sum();
                    ui.text(&format!("Shipped: {shipped}   Fish caught: {fish}   Dishes cooked: {cooked}   Monsters: {}", w.monsters_killed), Vec2::new(f.x + 40.0, f.y + f.h - 48.0), 2.0, ui::TEXT);
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
                    let o = opts_snapshot();
                    let labels = [format!("Zoom {:.0}%", o.0 * 100.0), format!("Music {:.0}%", o.1 * 100.0), format!("Sound {:.0}%", o.2 * 100.0)];
                    for ((minus, plus), label) in option_adjust_rects(sw, sh).iter().zip(labels.iter()) {
                        ui.button(*minus, "-", false);
                        ui.button(*plus, "+", false);
                        let m = ui.measure(label, 2.0);
                        ui.text(label, Vec2::new(minus.x + minus.w + (plus.x - minus.x - minus.w - m.x) / 2.0, minus.y + 20.0), 2.0, ui::TEXT);
                    }
                    ui.text("Key bindings (click a row, then press a key):", Vec2::new(f.x + 48.0, f.y + 226.0), 2.0, ui::TEXT);
                    let (binds, waiting) = binds_snapshot();
                    for (i, r) in binding_rects(sw, sh).iter().enumerate() {
                        ui.slot(*r, waiting == Some(i));
                        let keys = if waiting == Some(i) { "press a key...".to_string() } else { binds.get(i).map(|k| k.iter().map(|k| format!("{k:?}")).collect::<Vec<_>>().join(" / ")).unwrap_or_default() };
                        ui.text(&format!("{}: {keys}", input_map::BINDING_NAMES[i]), Vec2::new(r.x + 12.0, r.y + 14.0), 2.0, ui::TEXT);
                    }
                    ui.text("Mouse: left = use, right = act. F1 console, F3 debug, Ctrl+Tab switch game.", Vec2::new(f.x + 48.0, f.y + f.h - 36.0), 2.0, ui::TEXT);
                }
            }
        }
        Menu::Chest { tile, hovered } => {
            let f = menu_frame(sw, sh);
            ui.panel(f);
            let chest = w.container(*tile).cloned();
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
        Menu::Cooking { hovered } => {
            let f = menu_frame(sw, sh);
            ui.panel(f);
            ui.text("Cooking", Vec2::new(f.x + 16.0, f.y - 30.0), 3.0, ui::TEXT_LIGHT);
            let names = known_cooking(w);
            if names.is_empty() {
                ui.text("You don't know any recipes yet.", Vec2::new(f.x + 64.0, f.y + 64.0), 2.0, ui::TEXT);
            }
            for (i, (r, name)) in recipe_rects(sw, sh).iter().zip(&names).enumerate() {
                let recipe = &w.cooking[name];
                let can = recipe.can_craft_from(&w.items, &[&w.farmer.inventory, &w.farmer.fridge]);
                ui.slot(*r, *hovered == Some(i));
                let (tex, src) = item_sprite(rc, w, &recipe.output());
                let inner = FRect::new(r.x + 4.0, r.y + 4.0, r.w - 8.0, r.h - 8.0);
                if can {
                    ui.icon(tex, src, inner);
                } else {
                    ui.icon_tinted(tex, src, inner, Color::rgba(90, 90, 90, 140));
                }
            }
            if let Some(i) = hovered {
                if let Some(name) = names.get(*i) {
                    draw_recipe_tooltip(rc, ui, w, &w.cooking[name], sw);
                }
            }
        }
        Menu::Letter { letter, shown } => {
            let r = FRect::new(sw / 2.0 - 380.0, 40.0, 760.0, sh - 80.0);
            ui.fill(FRect::new(r.x + 8.0, r.y + 8.0, r.w, r.h), Color::rgba(0, 0, 0, 90));
            ui.out.fill(r, Color::rgb(250, 240, 215), ui.depth + 0.0001);
            ui.out.outline(r, 6.0, Color::rgb(200, 170, 120), ui.depth + 0.0002);
            let t: String = letter.text.chars().take(*shown as usize).collect();
            let mut y = r.y + 40.0;
            for line in ui.wrap(&t, r.w - 80.0, 2.0) {
                ui.text(&line, Vec2::new(r.x + 40.0, y), 2.0, ui::TEXT);
                y += 26.0;
            }
            let mut x = r.x + 40.0;
            for item in &letter.items {
                let slot = FRect::new(x, r.y + r.h - 110.0, 64.0, 64.0);
                ui.slot(slot, false);
                hud::draw_item_in_slot(rc, ui, w, item, slot);
                x += 76.0;
            }
            if letter.money > 0 {
                ui.text(&format!("+{}g", letter.money), Vec2::new(x + 10.0, r.y + r.h - 90.0), 3.0, ui::TEXT);
            }
            if let Some(title) = &letter.title {
                ui.text(title, Vec2::new(r.x + 40.0, r.y + r.h - 34.0), 2.0, Color::rgb(140, 100, 60));
            }
        }
        Menu::AnimalShop { list, hovered } => {
            let f = menu_frame(sw, sh);
            ui.panel(f);
            ui.text("Livestock", Vec2::new(f.x + 16.0, f.y - 30.0), 3.0, ui::TEXT_LIGHT);
            for (i, (r, (kind, price))) in shop_rows(sw, sh).iter().zip(list).enumerate() {
                ui.slot(*r, *hovered == Some(i));
                let data = w.animals_data.get(kind);
                if let Some(d) = data {
                    if let Some(tex) = rc.assets.try_tex(rc.cm, rc.textures, &d.texture.replace('\\', "/")) {
                        ui.icon(tex, IRect::new(0, 0, d.sprite_width.max(16), d.sprite_height.max(16)), FRect::new(r.x + 8.0, r.y + 4.0, 68.0, 68.0));
                    }
                }
                let name = data.map(|d| d.display_name.clone()).filter(|n| !n.is_empty()).unwrap_or_else(|| kind.clone());
                let house = data.map(|d| d.house.clone()).unwrap_or_default();
                ui.text(&name, Vec2::new(r.x + 96.0, r.y + 14.0), 3.0, ui::TEXT);
                ui.text(&format!("Lives in: {house}"), Vec2::new(r.x + 96.0, r.y + 46.0), 2.0, ui::TEXT);
                let ps = format!("{price}g");
                let m = ui.measure(&ps, 3.0);
                ui.text(&ps, Vec2::new(r.x + r.w - m.x - 24.0, r.y + 26.0), 3.0, if *price > w.farmer.money { Color::RED } else { ui::TEXT });
            }
            ui.text(&format!("Money: {}g", w.farmer.money), Vec2::new(f.x + 32.0, f.y + f.h - 50.0), 2.0, ui::TEXT);
        }
        Menu::Carpenter { list, hovered } => {
            let f = menu_frame(sw, sh);
            ui.panel(f);
            ui.text("Construction", Vec2::new(f.x + 16.0, f.y - 30.0), 3.0, ui::TEXT_LIGHT);
            for (i, (r, kind)) in shop_rows(sw, sh).iter().zip(list).enumerate() {
                ui.slot(*r, *hovered == Some(i));
                if kind == valley_sim::world::HOUSE_UPGRADE_ENTRY {
                    ui.text("Upgrade Farmhouse", Vec2::new(r.x + 96.0, r.y + 10.0), 3.0, ui::TEXT);
                    if let Some((money, mat, n)) = w.house_upgrade_cost() {
                        let name = w.items.object(&mat).map(|o| o.display_name.clone()).unwrap_or(mat);
                        ui.text(&format!("{money}g + {n} {name}  (3 days)"), Vec2::new(r.x + 96.0, r.y + 46.0), 2.0, ui::TEXT);
                    }
                    continue;
                }
                let Some(d) = w.buildings_data.get(kind) else { continue };
                if let Some(tex) = rc.assets.try_tex(rc.cm, rc.textures, &d.texture.replace('\\', "/")) {
                    let (tw, th) = rc.textures.size(tex);
                    let src = d.source_rect.as_ref().map(|s| IRect::new(s.x, s.y, s.width, s.height)).unwrap_or(IRect::new(0, 0, tw as i32, th as i32));
                    ui.icon(tex, src, FRect::new(r.x + 8.0, r.y + 4.0, 68.0, 68.0));
                }
                ui.text(&d.name, Vec2::new(r.x + 96.0, r.y + 10.0), 3.0, ui::TEXT);
                let mats: Vec<String> = d.build_materials.iter().map(|m| {
                    let (_, id) = valley_sim::item::split_qualified(&m.item_id);
                    format!("{} x{}", w.items.object(id).map(|o| o.display_name.clone()).unwrap_or_else(|| id.to_string()), m.amount)
                }).collect();
                ui.text(&format!("{}  {}", mats.join(", "), if d.build_days > 0 { format!("({} days)", d.build_days) } else { String::new() }), Vec2::new(r.x + 96.0, r.y + 46.0), 2.0, ui::TEXT);
                let ps = format!("{}g", d.build_cost);
                let m = ui.measure(&ps, 3.0);
                ui.text(&ps, Vec2::new(r.x + r.w - m.x - 24.0, r.y + 26.0), 3.0, if d.build_cost > w.farmer.money { Color::RED } else { ui::TEXT });
            }
        }
        Menu::Bundles { area, hovered } => {
            let f = menu_frame(sw, sh);
            let areas = w.bundle_areas();
            for (i, (r, a)) in bundle_tab_rects(sw, sh, areas.len()).iter().zip(&areas).enumerate() {
                let done = w.farmer.bundles.areas_completed.contains(a);
                ui.button(*r, &if done { format!("{a} *") } else { a.clone() }, i == *area);
            }
            ui.panel(f);
            if areas.is_empty() {
                ui.text("There are no bundles.", Vec2::new(f.x + 48.0, f.y + 60.0), 2.0, ui::TEXT);
            }
            let list: Vec<_> = w.bundles.iter().filter(|b| areas.get(*area) == Some(&b.area)).collect();
            for (bi, b) in list.iter().enumerate() {
                let done = w.farmer.bundles.completed.contains(&b.key);
                let y = f.y + 40.0 + bi as f32 * 84.0;
                ui.text(&b.name, Vec2::new(f.x + 32.0, y + 8.0), 2.0, if done { Color::rgb(60, 140, 60) } else { ui::TEXT });
                let filled = b.reqs.iter().enumerate().filter(|(i, _)| w.bundle_filled(&b.key, *i)).count();
                ui.text(&format!("{filled}/{}", b.num_required), Vec2::new(f.x + 32.0, y + 34.0), 2.0, ui::TEXT);
                for (ri, req) in b.reqs.iter().enumerate() {
                    let r = bundle_req_rect(sw, sh, bi, ri);
                    let have = w.bundle_filled(&b.key, ri) || done;
                    ui.slot(r, *hovered == Some((bi, ri)));
                    if req.item_id == "-1" {
                        ui.text(&format!("{}g", req.count), Vec2::new(r.x + 2.0, r.y + 18.0), 1.5, if have { Color::rgb(60, 140, 60) } else { ui::TEXT });
                        continue;
                    }
                    let item = valley_sim::item::Item::object_q(&req.item_id, req.count, req.min_quality);
                    let (tex, src) = item_sprite(rc, w, &item);
                    let inner = FRect::new(r.x + 4.0, r.y + 4.0, r.w - 8.0, r.h - 8.0);
                    if have {
                        ui.icon(tex, src, inner);
                    } else {
                        ui.icon_tinted(tex, src, inner, Color::rgba(60, 60, 60, 120));
                    }
                    ui.stack_number(req.count, r);
                }
            }
            if let Some((bi, ri)) = hovered {
                if let Some(req) = list.get(*bi).and_then(|b| b.reqs.get(*ri)) {
                    if req.item_id != "-1" {
                        let item = valley_sim::item::Item::object_q(&req.item_id, req.count, req.min_quality);
                        draw_tooltip(ui, w, &item, sw, sh);
                    }
                }
            }
        }
        Menu::Museum { hovered } => {
            let f = menu_frame(sw, sh);
            ui.panel(f);
            ui.text("Museum Donations", Vec2::new(f.x + 16.0, f.y - 30.0), 3.0, ui::TEXT_LIGHT);
            ui.text(&format!("Donated so far: {}", w.farmer.bundles.museum.len()), Vec2::new(f.x + 48.0, f.y + 40.0), 3.0, ui::TEXT);
            let mut x = f.x + 48.0;
            let mut y = f.y + 90.0;
            for id in &w.farmer.bundles.museum {
                let item = valley_sim::item::Item::object(id, 1);
                let (tex, src) = item_sprite(rc, w, &item);
                ui.icon(tex, src, FRect::new(x, y, 48.0, 48.0));
                x += 56.0;
                if x > f.x + f.w - 100.0 {
                    x = f.x + 48.0;
                    y += 56.0;
                }
            }
            ui.text("Click an item below to donate it:", Vec2::new(f.x + 32.0, f.y + 360.0), 2.0, ui::TEXT);
            for (i, r) in shop_inventory_rects(sw, sh).iter().enumerate() {
                let item = w.farmer.inventory.get(i);
                let ok = item.is_some_and(|it| w.museum_donatable(it));
                ui.slot(*r, *hovered == Some(i) && ok);
                if let Some(it) = item {
                    hud::draw_item_in_slot(rc, ui, w, it, *r);
                    if !ok {
                        ui.out.fill(*r, Color::rgba(60, 40, 30, 120), ui.depth + 0.001);
                    }
                }
            }
        }
        Menu::Blacksmith { hovered } => {
            let f = menu_frame(sw, sh);
            ui.panel(f);
            ui.text("Forge", Vec2::new(f.x + 16.0, f.y - 30.0), 3.0, ui::TEXT_LIGHT);
            for (i, (r, slot)) in shop_rows(sw, sh).iter().zip(upgradable_tools(w)).enumerate() {
                ui.slot(*r, *hovered == Some(i));
                let Some(item) = w.farmer.inventory.get(slot) else { continue };
                let (tex, src) = item_sprite(rc, w, item);
                ui.icon(tex, src, FRect::new(r.x + 8.0, r.y + 6.0, 64.0, 64.0));
                ui.text(&format!("Upgrade {}", w.items.display_name(item)), Vec2::new(r.x + 96.0, r.y + 12.0), 3.0, ui::TEXT);
                if let Some((money, bar, n)) = item.tool().and_then(|t| w.upgrade_cost(t.level)) {
                    let bar_name = bar.as_deref().and_then(|b| w.items.object(b)).map(|o| o.display_name.clone()).unwrap_or_else(|| "bars".into());
                    ui.text(&format!("{money}g + {n} {bar_name}  (2 days)"), Vec2::new(r.x + 96.0, r.y + 48.0), 2.0, ui::TEXT);
                }
            }
            if let Some((t, d)) = &w.farmer.tool_upgrade {
                ui.text(&format!("Working on: {} ({} day(s) left)", w.items.display_name(&valley_sim::item::Item::Tool(t.clone())), d.max(&0)), Vec2::new(f.x + 32.0, f.y + f.h - 140.0), 2.0, ui::TEXT);
            }
            ui.button(blacksmith_geode_rect(sw, sh), "Crack held geode (25g)", false);
        }
        Menu::LevelUp { skill, level, hovered } => {
            let f = menu_frame(sw, sh);
            ui.panel(f);
            ui.text_centered(&format!("{skill:?} Level {level}"), sw / 2.0, f.y + 40.0, 4.0, ui::TEXT);
            let choices = if *level == 5 || *level == 10 { valley_sim::professions::choices(*skill, *level, &w.farmer.professions) } else { vec![] };
            if choices.is_empty() {
                ui.text_centered("Your skills are improving.", sw / 2.0, f.y + 140.0, 3.0, ui::TEXT);
                ui.text_centered("(click to continue)", sw / 2.0, f.y + f.h - 60.0, 2.0, ui::TEXT);
            } else {
                ui.text_centered("Choose a profession:", sw / 2.0, f.y + 100.0, 3.0, ui::TEXT);
                for (i, (r, p)) in levelup_choice_rects(sw, sh).iter().zip(&choices).enumerate() {
                    ui.slot(*r, *hovered == Some(i));
                    ui.text_centered(p.name, r.x + r.w / 2.0, r.y + 30.0, 3.0, ui::TEXT);
                    for (k, line) in ui.wrap(p.description, r.w - 40.0, 2.0).iter().enumerate() {
                        ui.text(line, Vec2::new(r.x + 20.0, r.y + 90.0 + k as f32 * 26.0), 2.0, ui::TEXT);
                    }
                }
            }
        }
        Menu::OrderBoard => {
            let f = menu_frame(sw, sh);
            ui.panel(f);
            ui.text_centered("Special Orders", sw / 2.0, f.y + 28.0, 3.0, ui::TEXT);
            let half = (f.w - 96.0) / 2.0;
            let card = |i: usize| FRect::new(f.x + 32.0 + i as f32 * (half + 32.0), f.y + 80.0, half, f.h - 112.0);
            if let Some(o) = &w.farmer.special_order {
                // The active order and its progress.
                let r = card(0);
                ui.slot(FRect::new(r.x, r.y, f.w - 64.0, r.h), false);
                ui.text(&o.name, Vec2::new(r.x + 20.0, r.y + 18.0), 3.0, ui::TEXT);
                let mut y = r.y + 60.0;
                for line in ui.wrap(&o.text, f.w - 104.0, 2.0) {
                    ui.text(&line, Vec2::new(r.x + 20.0, y), 2.0, ui::TEXT);
                    y += 26.0;
                }
                y += 16.0;
                for ob in &o.objectives {
                    let mark = if ob.done() { "[x]" } else { "[ ]" };
                    ui.text(&format!("{mark} {} ({}/{})", ob.text, ob.progress, ob.required), Vec2::new(r.x + 20.0, y), 2.0, ui::TEXT);
                    y += 30.0;
                }
                ui.text(&format!("Days left: {}", o.days_left), Vec2::new(r.x + 20.0, r.y + r.h - 40.0), 2.0, ui::TEXT);
            } else if w.order_offers.is_empty() {
                ui.text_centered("No orders posted this week.", sw / 2.0, f.y + 200.0, 2.0, ui::TEXT);
            } else {
                for (i, id) in w.order_offers.iter().enumerate().take(2) {
                    let r = card(i);
                    ui.slot(r, false);
                    let Some((name, text)) = w.order_text(id) else { continue };
                    for (k, line) in ui.wrap(&name, r.w - 40.0, 3.0).iter().enumerate().take(2) {
                        ui.text(line, Vec2::new(r.x + 20.0, r.y + 18.0 + k as f32 * 30.0), 3.0, ui::TEXT);
                    }
                    for (k, line) in ui.wrap(&text, r.w - 40.0, 2.0).iter().enumerate().take(9) {
                        ui.text(line, Vec2::new(r.x + 20.0, r.y + 90.0 + k as f32 * 26.0), 2.0, ui::TEXT);
                    }
                    if let Some(d) = w.special_orders.get(id) {
                        let days = valley_sim::special_orders::duration_days(&d.duration);
                        ui.text(&format!("{days} days"), Vec2::new(r.x + 20.0, r.y + r.h - 100.0), 2.0, ui::TEXT);
                    }
                    ui.button(order_accept_rects(sw, sh)[i], "Accept", false);
                }
            }
        }
        Menu::Billboard => {
            let f = menu_frame(sw, sh);
            ui.panel(f);
            ui.text(&format!("{} - Year {}", w.clock.season.name(), w.clock.year), Vec2::new(f.x + 32.0, f.y + 28.0), 3.0, ui::TEXT);
            // Calendar grid: 4 weeks x 7 days.
            for d in 0..28 {
                let cell = FRect::new(f.x + 32.0 + (d % 7) as f32 * 68.0, f.y + 70.0 + (d / 7) as f32 * 62.0, 64.0, 58.0);
                ui.slot(cell, d + 1 == w.clock.day);
                ui.text(&(d + 1).to_string(), Vec2::new(cell.x + 4.0, cell.y + 4.0), 2.0, ui::TEXT);
                for npc in &w.npcs {
                    if let Some((season, day)) = &npc.birthday {
                        if valley_sim::time::Season::parse(season) == Some(w.clock.season) && *day == d + 1 {
                            let initial: String = npc.display_name.chars().take(3).collect();
                            ui.text(&initial, Vec2::new(cell.x + 6.0, cell.y + 32.0), 2.0, Color::rgb(200, 60, 90));
                        }
                    }
                }
            }
            let qx = f.x + 540.0;
            ui.text("Help Wanted", Vec2::new(qx, f.y + 70.0), 3.0, ui::TEXT);
            match &w.daily_quest {
                Some(q) => {
                    for (i, line) in ui.wrap(&q.description, f.w - 580.0, 2.0).iter().enumerate() {
                        ui.text(line, Vec2::new(qx, f.y + 120.0 + i as f32 * 26.0), 2.0, ui::TEXT);
                    }
                    ui.text(&format!("Reward: {}g", q.reward_money), Vec2::new(qx, f.y + 300.0), 2.0, ui::TEXT);
                    ui.button(billboard_accept_rect(sw, sh), "Accept", false);
                }
                None => ui.text("No requests today.", Vec2::new(qx, f.y + 120.0), 2.0, ui::TEXT),
            }
        }
        Menu::Journal { selected } => {
            let f = menu_frame(sw, sh);
            ui.panel(f);
            ui.text("Journal", Vec2::new(f.x + 16.0, f.y - 30.0), 3.0, ui::TEXT_LIGHT);
            if let Some(o) = &w.farmer.special_order {
                let progress: Vec<String> = o.objectives.iter().map(|ob| format!("{} {}/{}", ob.text, ob.progress, ob.required)).collect();
                let line = format!("Special order: {} - {} ({} days left)", o.name, progress.join(", "), o.days_left);
                ui.text(&line, Vec2::new(f.x + 32.0, f.y + f.h - 36.0), 2.0, Color::rgb(120, 60, 120));
            }
            if w.farmer.quests.is_empty() {
                ui.text("No active quests.", Vec2::new(f.x + 48.0, f.y + 60.0), 2.0, ui::TEXT);
            }
            for (i, (r, q)) in journal_rows(sw, sh).iter().zip(&w.farmer.quests).enumerate() {
                ui.slot(*r, i == *selected);
                ui.text(&q.title, Vec2::new(r.x + 12.0, r.y + 16.0), 2.0, ui::TEXT);
            }
            if let Some(q) = w.farmer.quests.get(*selected) {
                let x = f.x + 420.0;
                let mut y = f.y + 48.0;
                ui.text(&q.title, Vec2::new(x, y), 3.0, ui::TEXT);
                y += 44.0;
                for line in ui.wrap(&q.description, f.w - 460.0, 2.0) {
                    ui.text(&line, Vec2::new(x, y), 2.0, ui::TEXT);
                    y += 26.0;
                }
                y += 16.0;
                for line in ui.wrap(&q.hint, f.w - 460.0, 2.0) {
                    ui.text(&line, Vec2::new(x, y), 2.0, Color::rgb(120, 80, 40));
                    y += 26.0;
                }
                if let Some(p) = q.progress_text() {
                    ui.text(&format!("Progress: {p}"), Vec2::new(x, y + 16.0), 2.0, ui::TEXT);
                }
                if q.days_left > 0 {
                    ui.text(&format!("Days left: {}", q.days_left), Vec2::new(x, f.y + f.h - 80.0), 2.0, ui::TEXT);
                }
                if q.reward_money > 0 {
                    ui.text(&format!("Reward: {}g", q.reward_money), Vec2::new(x, f.y + f.h - 50.0), 2.0, ui::TEXT);
                }
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
    static OPTS: std::cell::Cell<(f32, f32, f32)> = const { std::cell::Cell::new((1.0, 0.75, 1.0)) };
    static BINDS: std::cell::RefCell<(Vec<Vec<Key>>, Option<usize>)> = const { std::cell::RefCell::new((Vec::new(), None)) };
}

fn binds_snapshot() -> (Vec<Vec<Key>>, Option<usize>) {
    BINDS.with(|b| b.borrow().clone())
}

/// (zoom, music, sound) for drawing the options tab.
fn opts_snapshot() -> (f32, f32, f32) {
    OPTS.with(|o| o.get())
}

fn ui_mouse_hint() -> Vec2 {
    let (x, y) = MOUSE.with(|m| m.get());
    Vec2::new(x, y)
}

impl ValleyGame {
    /// Debug console commands (also used by headless scripts).
    pub fn run_command(&mut self, cmd: &str) -> String {
        let t: Vec<&str> = cmd.split_whitespace().collect();
        let Some(w) = self.world.as_mut() else { return "no game loaded".into() };
        let num = |i: usize| t.get(i).and_then(|v| v.parse::<i32>().ok());
        match t.first().copied() {
            Some("warp") if t.len() >= 2 => {
                let (x, y) = (num(2).unwrap_or(5), num(3).unwrap_or(5));
                w.warp_farmer(t[1], TilePos::new(x, y), None);
                format!("warped to {}", w.current)
            }
            Some("mine") => {
                w.enter_mine_level(num(1).unwrap_or(1));
                format!("entered {}", w.current)
            }
            Some("give") if t.len() >= 2 => {
                let n = num(2).unwrap_or(1);
                match w.items.create(t[1], n) {
                    Some(item) => {
                        w.give_item(item);
                        format!("gave {} x{n}", t[1])
                    }
                    None => format!("unknown item {}", t[1]),
                }
            }
            Some("money") => {
                w.farmer.money = num(1).unwrap_or(w.farmer.money);
                format!("money = {}", w.farmer.money)
            }
            Some("time") => {
                w.clock.time = num(1).unwrap_or(w.clock.time).clamp(600, 2550);
                format!("time = {}", w.clock.time)
            }
            Some("child") => w.debug_add_child(num(1).unwrap_or(0)),
            Some("wear") if t.len() >= 2 => match w.items.create(t[1], 1) {
                Some(item @ valley_sim::item::Item::Clothing { .. }) => {
                    let slot = w.farmer.inventory.slots.iter().position(|s| s.is_none());
                    match slot {
                        Some(i) => {
                            w.farmer.inventory.slots[i] = Some(item);
                            w.equip(i);
                            format!("wearing {}", t[1])
                        }
                        None => "inventory full".into(),
                    }
                }
                _ => format!("{} isn't clothing", t[1]),
            },
            Some("day") => {
                w.end_day(false);
                format!("now day {} of {}", w.clock.day, w.clock.season.name())
            }
            Some("slot") => {
                w.farmer.current_slot = num(1).unwrap_or(0).clamp(0, 35) as usize;
                format!("slot = {}", w.farmer.current_slot)
            }
            Some("tile") => format!("{} {:?} facing {:?}", w.current, w.farmer.tile(), w.farmer.facing),
            Some("bundles") => {
                w.emit(valley_sim::world::WorldEvent::OpenBundles);
                "opening bundles".into()
            }
            _ => "commands: warp <loc> <x> <y> | mine <n> | give <id> [n] | money <n> | time <hhmm> | day | child [age] | wear <(H|S|P)id> | bundles | slot <n> | tile".into(),
        }
    }
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
        OPTS.with(|o| o.set((self.options.zoom, self.options.music_volume, self.options.sound_volume)));
        BINDS.with(|b| *b.borrow_mut() = ((0..input_map::BINDING_NAMES.len()).map(|i| self.options.bindings.get(i).clone()).collect(), self.rebinding));
        self.fade.alpha += (self.fade.target - self.fade.alpha).clamp(-0.05, 0.05);
        self.flash = self.flash.saturating_sub(1);
        if self.fade.target >= 1.0 && self.fade.alpha >= 1.0 {
            self.fade.target = 0.0;
        }
        match self.screen {
            Screen::Title { .. } => self.update_title(ctx),
            Screen::LoadList { .. } => self.update_load_list(ctx),
            Screen::Create { .. } => self.update_create(ctx),
            Screen::Playing => self.update_playing(ctx),
        }
        self.update_music(ctx);
    }

    fn suspend(&mut self, ctx: &mut EngineCtx) {
        ctx.audio.stop_music();
        self.music = None;
    }

    fn command(&mut self, _ctx: &mut EngineCtx, cmd: &str) -> String {
        self.run_command(cmd)
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
        if self.flash > 0 {
            out.fill(FRect::new(0.0, 0.0, ctx.screen_w as f32, ctx.screen_h as f32), Color::rgba(255, 255, 255, (self.flash * 15).min(200) as u8), 99.0);
        }
    }
}
