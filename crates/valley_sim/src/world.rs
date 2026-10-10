//! The world: owns all simulation state and advances it tick by tick.

use std::collections::{BTreeMap, HashMap, VecDeque};

use engine::{IRect, Rng, Vec2};
use valley_content::data::{CharacterData, LocationData, ShopData};
use valley_content::ContentManager;

use crate::collision::{box_blocked, tile_passable};
use crate::crop::Crop;
use crate::dialogue::{self, DialoguePage, Substitutions};
use crate::farmer::{Farmer, Skill};
use crate::gsq::QueryCtx;
use crate::input::PlayerInput;
use crate::item::{Item, ItemDb};
use crate::location::GameLocation;
use crate::npc::{Npc, NpcHome, RouteLeg};
use crate::objects::{DebrisKind, ObjKind, PlacedObject};
use crate::pathfind;
use crate::schedule;
use crate::shop::{self, ShopStock};
use crate::terrain::{TerrainFeature, Tree};
use crate::time::{Clock, ClockEvent, Weather, DAY_END};
use crate::{Dir, TilePos, TILE};

/// Carpenter menu entry meaning "upgrade the farmhouse".
pub const HOUSE_UPGRADE_ENTRY: &str = "__farmhouse_upgrade";

/// Milliseconds per simulation tick.
pub const TICK_MS: u32 = 1000 / engine::TICKS_PER_SECOND;

#[derive(Clone, Debug, PartialEq)]
pub struct DayReport {
    pub shipped: Vec<(Item, i32)>,
    pub earned: i32,
    pub passed_out: bool,
    pub money_lost: i32,
}

/// Things the front end should react to (sounds, UI, transitions).
/// Minutes a paid trip (Port.Travel) takes.
pub const TRAVEL_MINUTES: i32 = 30;

#[derive(Clone, Debug, PartialEq)]
pub enum WorldEvent {
    Sound(String),
    Hud(String),
    Pickup(Item),
    Dialogue { npc: Option<String>, pages: Vec<DialoguePage> },
    Message(String),
    OpenShop(ShopStock),
    /// Show the special orders board.
    OpenOrderBoard,
    /// Ask before jumping down a deep-cavern shaft.
    ShaftPrompt,
    /// Offer a paid trip (bus, boat...) to another location.
    TravelPrompt { location: String, tile: TilePos, fare: i32 },
    Warped { from: String, to: String },
    /// Farmer stepped into bed: ask whether to sleep.
    SleepPrompt,
    /// Farmer used an edible item: ask whether to eat it.
    EatPrompt { slot: usize },
    DayEnded(DayReport),
    LevelUp(Skill, i32),
    Exhausted,
    /// Open the chest at this tile in the current location.
    OpenChest(TilePos),
    /// Choose a mine floor (multiples of 5 up to `deepest`).
    ElevatorMenu { deepest: i32 },
    OpenCooking,
    OpenBillboard,
    /// Show a letter from the mailbox.
    Letter { id: String, letter: crate::mail::Letter },
    QuestCompleted(String),
    NewQuest(String),
    /// (animal kind, price)
    OpenAnimalShop(Vec<(String, i32)>),
    /// Building kinds available to construct.
    OpenCarpenter(Vec<String>),
    Music(String),
    Fade { to_black: bool },
    EventEnded,
    OpenBlacksmith,
    OpenBundles,
    OpenMuseum,
    /// Talked to the festival host: ask to start the main event.
    FestivalHost { festival: String },
    /// An NPC asked a question; answer with `World::answer_question`.
    DialogueQuestion { npc: String, prompt: String, choices: Vec<String> },
    /// Lightning flash.
    Lightning,
}

/// An item lying in the world (tool drops pop out and fly to the farmer).
#[derive(Clone, Debug, PartialEq)]
pub struct DroppedItem {
    pub location: String,
    pub item: Item,
    pub pos: Vec2,
    pub vel: Vec2,
    /// Height above ground (for the pop arc).
    pub z: f32,
    pub vz: f32,
    pub age_ms: u32,
}

pub struct NewGame {
    pub appearance: crate::farmer::Appearance,
    pub farmer_name: String,
    pub farm_name: String,
    pub favorite_thing: String,
    pub male: bool,
    pub seed: u64,
}

impl Default for NewGame {
    fn default() -> Self {
        Self { appearance: Default::default(), farmer_name: "Farmer".into(), farm_name: "Sunny".into(), favorite_thing: "Rust".into(), male: true, seed: 12345 }
    }
}

pub struct World {
    pub items: ItemDb,
    pub clock: Clock,
    pub weather: Weather,
    pub weather_tomorrow: Weather,
    pub daily_luck: f64,
    pub farmer: Farmer,
    pub locations: BTreeMap<String, GameLocation>,
    pub location_data: BTreeMap<String, LocationData>,
    pub current: String,
    pub npcs: Vec<Npc>,
    pub shipping_bin: Vec<Item>,
    pub dropped: Vec<DroppedItem>,
    pub seed: u64,
    pub rng: Rng,
    pub events: Vec<WorldEvent>,
    /// Total simulated milliseconds (drives animations).
    pub game_ms: u64,
    pub shops: BTreeMap<String, ShopData>,
    pub gift_tastes: HashMap<String, String>,
    /// When true, the clock and NPCs stop (menus/dialogue open).
    pub paused: bool,
    location_edges: HashMap<String, Vec<String>>,
    /// Bed tile in the farmhouse used for waking up.
    pub bed_tile: TilePos,
    /// Ticks since the farmer last left a bed tile (avoids re-prompting).
    in_bed: bool,
    pub last_report: Option<DayReport>,
    /// Raw schedule tables per NPC name.
    pub schedule_data: HashMap<String, HashMap<String, String>>,
    pub recipes: BTreeMap<String, crate::crafting::Recipe>,
    pub fish: BTreeMap<String, crate::fishing::FishInfo>,
    pub fishing: crate::fishing::FishingState,
    pub monster_info: BTreeMap<String, crate::combat::MonsterInfo>,
    pub monsters: Vec<crate::combat::Monster>,
    pub next_monster_id: u32,
    pub monsters_killed: u32,
    pub floaters: Vec<crate::combat::Floater>,
    pub weapon_swing: Option<crate::combat::WeaponSwing>,
    pub special: Option<crate::arsenal::SpecialMove>,
    pub special_cooldown_ms: u32,
    pub sling: Option<crate::arsenal::SlingPull>,
    pub projectiles: Vec<crate::arsenal::Projectile>,
    /// Limited-stock purchases today, keyed "shop:(T)id".
    pub shop_sold: BTreeMap<String, i32>,
    /// Data/SpecialOrders and this week's board offers.
    pub special_orders: BTreeMap<String, crate::special_orders::SpecialOrderData>,
    pub order_offers: Vec<String>,
    /// The spouse will ask about a baby when next spoken to.
    pub child_question_pending: bool,
    /// Level maps from content (Maps/Mines/1..), empty = generate caves.
    pub mine_maps: Vec<valley_content::Map>,
    pub mine_items: crate::mines::MineItems,
    pub mine_stones_left: i32,
    pub mine_ladder_spawned: bool,
    /// Where the up-ladder returns to.
    pub mine_entrance: (String, TilePos),
    pub cooking: BTreeMap<String, crate::crafting::Recipe>,
    pub mail_data: HashMap<String, String>,
    pub quest_data: HashMap<String, String>,
    /// Today's help-wanted request (if any, not yet accepted).
    pub daily_quest: Option<crate::quests::Quest>,
    pub buildings_data: BTreeMap<String, valley_content::data::BuildingData>,
    pub animals_data: BTreeMap<String, valley_content::data::FarmAnimalData>,
    pub building_maps: crate::farm::BuildingMaps,
    pub buildings: Vec<crate::farm::Building>,
    pub animals: Vec<crate::farm::FarmAnimal>,
    pub next_building_id: u32,
    pub next_animal_id: u32,
    /// Item id of hay (well-known id or placeholder).
    pub hay_id: Option<String>,
    /// Event scripts per location (key -> script).
    pub event_data: HashMap<String, HashMap<String, String>>,
    pub event: Option<crate::events::EventRunner>,
    pub compat: Compat,
    pub bundles: Vec<crate::bundles::BundleDef>,
    pub museum_rewards: BTreeMap<String, crate::bundles::MuseumReward>,
    /// Festivals keyed by date ("fall16").
    pub festivals: HashMap<String, crate::festivals::FestivalInfo>,
    /// Temporary maps used by event scripts (changeToTemporaryMap).
    pub festival_maps: HashMap<String, valley_content::Map>,
    pub festival_done_today: bool,
    /// MarriageDialogue<Name> tables.
    pub spouse_dialogue: HashMap<String, HashMap<String, String>>,
    pub pending_question: Option<(String, dialogue::Question)>,
    pub fruit_trees: BTreeMap<String, valley_content::data::FruitTreeData>,
    pub wild_trees: BTreeMap<String, valley_content::data::WildTreeData>,
    pub bombs: Vec<crate::equipment::LitBomb>,
    pub explosions: Vec<crate::equipment::Explosion>,
    pub trap_fish: Vec<crate::equipment::TrapFish>,
    /// Farmhouse maps by upgrade level (1.., from Maps/FarmHouse<N>).
    pub house_maps: BTreeMap<i32, valley_content::Map>,
    pub pet: Option<crate::pets::Pet>,
    /// Pet sheet assets that exist in content.
    pub pet_textures: Vec<String>,
    pub garbage: Option<crate::pets::GarbageData>,
    /// Trash cans searched today ("Location:id").
    pub garbage_searched: Vec<String>,
}

/// Fallbacks for content maps that don't carry this engine's helper
/// properties (approximate well-known positions on the standard farm).
#[derive(Clone, Debug, Default)]
pub struct Compat {
    pub shipping_bin: Option<TilePos>,
    pub farmhouse_door: Option<TilePos>,
    pub farmhouse_entry: Option<TilePos>,
    /// No "Bed" tile in the farmhouse map: sleep from the wake spot.
    pub bed_fallback: bool,
}

impl World {
    pub fn new(cm: &mut ContentManager, opts: NewGame) -> Self {
        let items = ItemDb::load(cm);
        let location_data = cm.load_dict::<LocationData>("Data/Locations");
        let mut locations = BTreeMap::new();
        for (name, data) in &location_data {
            let Some(create) = &data.create_on_load else { continue };
            if create.map_path.is_empty() {
                continue;
            }
            if let Some(map) = cm.load_map(&create.map_path) {
                let mut loc = GameLocation::new(name, map);
                if let Some(dn) = &data.display_name {
                    loc.display_name = cm.expand(dn);
                }
                locations.insert(name.clone(), loc);
            }
        }
        // Core locations may be created by code rather than data.
        for name in ["Farm", "FarmHouse"] {
            if !locations.contains_key(name) {
                if let Some(map) = cm.load_map(&format!("Maps/{name}")) {
                    locations.insert(name.to_string(), GameLocation::new(name, map));
                }
            }
        }
        let shops = cm.load_dict::<ShopData>("Data/Shops");
        let gift_tastes = cm.load_string_dict("Data/NPCGiftTastes");

        let mut farmer = Farmer::new(&opts.farmer_name, &opts.farm_name);
        farmer.favorite_thing = opts.favorite_thing.clone();
        farmer.appearance = opts.appearance.clone();
        farmer.male = opts.male;

        let mut w = World {
            items,
            clock: Clock::default(),
            weather: Weather::Sun,
            weather_tomorrow: Weather::Sun,
            daily_luck: 0.0,
            farmer,
            locations,
            location_data,
            current: "FarmHouse".into(),
            npcs: Vec::new(),
            shipping_bin: Vec::new(),
            dropped: Vec::new(),
            seed: opts.seed,
            rng: Rng::new(opts.seed),
            events: Vec::new(),
            game_ms: 0,
            shops,
            gift_tastes,
            paused: false,
            location_edges: HashMap::new(),
            bed_tile: TilePos::new(0, 0),
            in_bed: false,
            last_report: None,
            schedule_data: HashMap::new(),
            recipes: crate::crafting::parse_all(&cm.load_string_dict("Data/CraftingRecipes")),
            fish: cm.load_string_dict("Data/Fish").iter().filter_map(|(k, v)| crate::fishing::parse_fish(k, v).map(|f| (k.clone(), f))).collect(),
            fishing: crate::fishing::FishingState::Idle,
            monster_info: cm.load_string_dict("Data/Monsters").iter().filter_map(|(k, v)| crate::combat::parse_monster(k, v).map(|m| (k.clone(), m))).collect(),
            monsters: Vec::new(),
            next_monster_id: 0,
            monsters_killed: 0,
            floaters: Vec::new(),
            weapon_swing: None,
            special: None,
            special_cooldown_ms: 0,
            sling: None,
            projectiles: Vec::new(),
            shop_sold: BTreeMap::new(),
            special_orders: cm.load_json("Data/SpecialOrders").unwrap_or_default(),
            order_offers: Vec::new(),
            child_question_pending: false,
            mine_maps: (1..=40).map_while(|i| cm.load_map(&format!("Maps/Mines/{i}"))).collect(),
            mine_items: crate::mines::MineItems::default(),
            mine_stones_left: 0,
            mine_ladder_spawned: false,
            mine_entrance: ("Mine".into(), TilePos::new(5, 5)),
            cooking: crate::crafting::parse_all_cooking(&cm.load_string_dict("Data/CookingRecipes")),
            mail_data: cm.load_string_dict("Data/mail"),
            quest_data: cm.load_string_dict("Data/Quests"),
            daily_quest: None,
            buildings_data: cm.load_dict("Data/Buildings"),
            animals_data: cm.load_dict("Data/FarmAnimals"),
            building_maps: Default::default(),
            buildings: Vec::new(),
            animals: Vec::new(),
            next_building_id: 0,
            next_animal_id: 0,
            hay_id: None,
            event_data: HashMap::new(),
            event: None,
            compat: Compat::default(),
            bundles: Vec::new(),
            museum_rewards: BTreeMap::new(),
            festivals: HashMap::new(),
            festival_maps: HashMap::new(),
            festival_done_today: false,
            spouse_dialogue: HashMap::new(),
            pending_question: None,
            fruit_trees: cm.load_dict("Data/FruitTrees"),
            wild_trees: cm.load_dict("Data/WildTrees"),
            bombs: Vec::new(),
            explosions: Vec::new(),
            trap_fish: cm.load_string_dict("Data/Fish").iter().filter_map(|(k, v)| crate::equipment::parse_trap(k, v)).collect(),
            house_maps: (1..=3).filter_map(|i| cm.load_map(&format!("Maps/FarmHouse{i}")).map(|m| (i, m))).collect(),
            pet: None,
            pet_textures: ["Animals/dog", "Animals/Dev_Dog"].iter().filter(|t| cm.exists(t)).map(|s| s.to_string()).collect(),
            garbage: cm.load_json("Data/GarbageCans"),
            garbage_searched: Vec::new(),
        };
        w.trap_fish.sort_by(|a, b| a.id.cmp(&b.id));
        for key in cm.load_string_dict("Data/Festivals/FestivalDates").keys() {
            let data = cm.load_string_dict(&format!("Data/Festivals/{key}"));
            if let Some(f) = crate::festivals::parse_festival(key, data) {
                w.festivals.insert(key.clone(), f);
            }
        }
        // Preload temporary maps referenced by festival and event scripts.
        let scripts: Vec<String> = w
            .festivals
            .values()
            .flat_map(|f| f.data.values().cloned().collect::<Vec<_>>())
            .chain(w.event_data.values().flat_map(|d| d.values().cloned().collect::<Vec<_>>()))
            .collect();
        for sc in scripts {
            for c in crate::events::split_commands(&sc) {
                let t = crate::events::tokens(&c);
                if t.first().map(|s| s == "changeToTemporaryMap").unwrap_or(false) {
                    if let Some(name) = t.get(1) {
                        if !w.festival_maps.contains_key(name) {
                            if let Some(m) = cm.load_map(&format!("Maps/{name}")) {
                                w.festival_maps.insert(name.clone(), m);
                            }
                        }
                    }
                }
            }
        }
        w.bundles = crate::bundles::parse_bundles(&cm.load_string_dict("Data/Bundles"), &w.items);
        if let Some(v) = cm.load_json_value("Data/MuseumRewards") {
            w.museum_rewards = crate::bundles::parse_museum_rewards(&v);
        }
        let names: Vec<String> = w.locations.keys().cloned().collect();
        for n in names {
            let d = cm.load_string_dict(&format!("Data/Events/{n}"));
            if !d.is_empty() {
                w.event_data.insert(n, d);
            }
        }
        let interior_maps: Vec<String> = w.buildings_data.values().filter_map(|b| b.indoor_map.clone()).collect();
        for m in interior_maps {
            if let Some(map) = cm.load_map(&format!("Maps/{m}")) {
                w.building_maps.insert(m, map);
            }
        }
        w.hay_id = ["178", "Dev_Hay"].iter().find(|id| w.items.objects.contains_key(**id)).map(|s| s.to_string());
        w.mine_items = crate::mines::MineItems::resolve(&w.items);
        w.build_location_edges();
        w.bed_tile = w.find_bed_tile();
        w.setup_compat();
        w.spawn_npcs(cm);
        let romanceable: Vec<String> = w.npcs.iter().filter(|n| n.can_romance).map(|n| n.name.clone()).collect();
        for n in romanceable {
            let d = cm.load_string_dict(&format!("Characters/Dialogue/MarriageDialogue{n}"));
            if !d.is_empty() {
                w.spouse_dialogue.insert(n, d);
            }
        }
        w.populate_new_farm();
        w.adopt_pet("Biscuit");
        let mut grass_rng = Rng::new(Rng::seed_from(&[w.seed as i64, 88]));
        w.seed_grass(&mut grass_rng);
        w.farmer.mailbox.push("welcome".into());
        for starter in ["Dev_Q1"] {
            if w.quest_data.contains_key(starter) {
                w.add_quest(starter);
            }
        }
        let wake = w.wake_tile();
        w.current = if w.locations.contains_key("FarmHouse") { "FarmHouse".into() } else { w.locations.keys().next().cloned().unwrap_or_default() };
        w.farmer.set_tile(wake);
        w.in_bed = true;
        w.start_day();
        w
    }

    pub fn location(&self) -> &GameLocation {
        &self.locations[&self.current]
    }

    pub fn location_mut(&mut self) -> &mut GameLocation {
        self.locations.get_mut(&self.current).expect("current location")
    }

    pub fn emit(&mut self, e: WorldEvent) {
        if let WorldEvent::LevelUp(skill, level) = &e {
            self.farmer.pending_levelups.push((*skill, *level));
        }
        self.events.push(e);
    }

    pub fn sound(&mut self, cue: &str) {
        self.events.push(WorldEvent::Sound(cue.to_string()));
    }

    pub fn hud(&mut self, msg: impl Into<String>) {
        self.events.push(WorldEvent::Hud(msg.into()));
    }

    pub fn drain_events(&mut self) -> Vec<WorldEvent> {
        std::mem::take(&mut self.events)
    }

    pub(crate) fn day_rng(&self, salt: i64) -> Rng {
        Rng::new(Rng::seed_from(&[self.seed as i64, self.clock.days_played() as i64, salt]))
    }

    // ---------------------------------------------------------------- setup

    pub(crate) fn find_bed_tile_pub(&self) -> TilePos {
        self.find_bed_tile()
    }

    pub(crate) fn build_location_edges_pub(&mut self) {
        self.rebuild_location_edges();
    }

    fn find_bed_tile(&self) -> TilePos {
        if let Some(h) = self.locations.get("FarmHouse") {
            for y in 0..h.height() {
                for x in 0..h.width() {
                    let p = TilePos::new(x, y);
                    if h.tile_prop("Back", p, "Bed").is_some() {
                        return p;
                    }
                }
            }
        }
        TilePos::new(5, 5)
    }

    fn setup_compat(&mut self) {
        let has_bed = self.locations.get("FarmHouse").is_some_and(|h| {
            (0..h.height()).any(|y| (0..h.width()).any(|x| h.tile_prop("Back", TilePos::new(x, y), "Bed").is_some()))
        });
        self.compat.bed_fallback = !has_bed;
        let Some(farm) = self.locations.get("Farm") else { return };
        let has_door = door_warps(farm).iter().any(|(_, t, _)| t == "FarmHouse");
        let big = farm.width() >= 60;
        if farm.map_prop_tile("ShippingBinTile").is_none() && big {
            self.compat.shipping_bin = Some(TilePos::new(71, 14));
        }
        if !has_door && big {
            self.compat.farmhouse_door = Some(TilePos::new(64, 15));
            self.compat.farmhouse_entry = Some(TilePos::new(3, 11));
        }
    }

    /// Tile where the farmer wakes: next to the bed, or the arrival tile.
    pub fn wake_tile(&self) -> TilePos {
        let Some(house) = self.locations.get("FarmHouse") else { return TilePos::new(1, 1) };
        for d in [Dir::Left, Dir::Down, Dir::Right, Dir::Up] {
            let p = self.bed_tile.offset(d);
            if tile_passable(house, p) && house.tile_prop("Back", p, "Bed").is_none() {
                return p;
            }
        }
        self.location_data
            .get("FarmHouse")
            .and_then(|d| d.default_arrival_tile.as_ref())
            .map(|t| TilePos::new(t.x, t.y))
            .unwrap_or(TilePos::new(3, 3))
    }

    /// Recompute the warp graph and runtime blocked tiles (after buildings change).
    pub(crate) fn rebuild_location_edges(&mut self) {
        let tiles: std::collections::HashSet<TilePos> = self.buildings.iter().flat_map(|b| b.footprint().collect::<Vec<_>>()).collect();
        if let Some(farm) = self.locations.get_mut("Farm") {
            farm.blocked = tiles;
        }
        self.build_location_edges();
        // Building doors connect the farm and interiors for NPC routing.
        for b in self.buildings.clone() {
            if let Some(i) = &b.interior {
                self.location_edges.entry("Farm".into()).or_default().push(i.clone());
            }
        }
    }

    fn build_location_edges(&mut self) {
        let mut edges: HashMap<String, Vec<String>> = HashMap::new();
        for (name, loc) in &self.locations {
            let mut ns: Vec<String> = Vec::new();
            for w in &loc.warps {
                if self.locations.contains_key(&w.target) && !ns.contains(&w.target) {
                    ns.push(w.target.clone());
                }
            }
            for (_, target, _) in door_warps(loc) {
                if self.locations.contains_key(&target) && !ns.contains(&target) {
                    ns.push(target);
                }
            }
            edges.insert(name.clone(), ns);
        }
        self.location_edges = edges;
    }

    fn spawn_npcs(&mut self, cm: &mut ContentManager) {
        let chars = cm.load_dict::<CharacterData>("Data/Characters");
        for (name, data) in chars {
            let Some(home) = data.home.first() else { continue };
            if !self.locations.contains_key(&home.location) {
                continue;
            }
            let texture = format!("Characters/{}", data.texture_name.clone().unwrap_or_else(|| name.clone()));
            if !cm.exists(&texture) {
                continue;
            }
            let display = {
                let d = cm.expand(&data.display_name);
                if d.is_empty() { name.clone() } else { d }
            };
            let facing = Dir::parse(&home.direction).unwrap_or(Dir::Down);
            let mut npc = Npc {
                name: name.clone(),
                display_name: display,
                texture,
                portrait: format!("Portraits/{}", data.texture_name.clone().unwrap_or_else(|| name.clone())),
                location: home.location.clone(),
                pos: Vec2::ZERO,
                facing,
                home: NpcHome { location: home.location.clone(), tile: TilePos::new(home.tile.x, home.tile.y), facing },
                birthday: data.birth_season.clone().map(|s| (s, data.birth_day)),
                can_romance: data.can_be_romanced,
                dialogue: cm.load_string_dict(&format!("Characters/Dialogue/{name}")),
                schedule: Vec::new(),
                next_entry: 0,
                route: VecDeque::new(),
                end_facing: facing,
                moving: false,
                walk_ms: 0,
                bubble: None,
                pause_ms: 0,
            };
            npc.set_tile(npc.home.tile);
            let sched = cm.load_string_dict(&format!("Characters/schedules/{name}"));
            if !sched.is_empty() {
                self.schedule_data.insert(name.clone(), sched);
            }
            self.npcs.push(npc);
        }
    }

    /// Initial clutter on a new farm: weeds, stones, twigs and trees.
    fn populate_new_farm(&mut self) {
        let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, 77]));
        let items = self.items.clone();
        let Some(farm) = self.locations.get_mut("Farm") else { return };
        let (w, h) = (farm.width(), farm.height());
        let protected: Vec<TilePos> = ["FarmHouseEntry", "ShippingBinTile"].iter().filter_map(|k| farm.map_prop_tile(k)).collect();
        for y in 0..h {
            for x in 0..w {
                let p = TilePos::new(x, y);
                if !farm.is_clear(p) || !farm.is_diggable(p) {
                    continue;
                }
                if protected.iter().any(|q| q.manhattan(p) <= 3) {
                    continue;
                }
                let r = rng.next_f64();
                if r < 0.04 {
                    farm.state.objects.insert(p, debris_object_from(&items, &mut rng, DebrisKind::Weeds));
                } else if r < 0.06 {
                    farm.state.objects.insert(p, debris_object_from(&items, &mut rng, DebrisKind::Stone));
                } else if r < 0.075 {
                    farm.state.objects.insert(p, debris_object_from(&items, &mut rng, DebrisKind::Twig));
                } else if r < 0.085 {
                    let mut t = Tree::new(["1", "2", "3"][rng.range(0, 3) as usize], 5);
                    t.flip = rng.chance(0.5);
                    farm.state.terrain.insert(p, TerrainFeature::Tree(t));
                }
            }
        }
    }

    // ---------------------------------------------------------------- day cycle

    /// Morning setup: weather-dependent watering, NPC schedules, mail.
    pub fn start_day(&mut self) {
        self.festival_done_today = false;
        self.check_wedding();
        let raining = self.weather.is_raining();
        self.unlock_recipes();
        self.run_sprinklers();
        self.spawn_forage();
        self.deliver_dated_mail();
        self.roll_daily_quest();
        self.animals_morning();
        for loc in self.locations.values_mut() {
            if loc.outdoors && raining {
                for t in loc.state.terrain.values_mut() {
                    if let TerrainFeature::HoeDirt(d) = t {
                        d.watered = true;
                    }
                }
            }
        }
        let schedules = self.schedule_data.clone();
        let clock = self.clock.clone();
        for i in 0..self.npcs.len() {
            let name = self.npcs[i].name.clone();
            let hearts = self.farmer.hearts(&name);
            let npc = &mut self.npcs[i];
            npc.location = npc.home.location.clone();
            npc.set_tile(npc.home.tile);
            npc.facing = npc.home.facing;
            npc.route.clear();
            npc.next_entry = 0;
            npc.schedule = match schedules.get(&name) {
                Some(s) => schedule::pick_key(s, &clock, hearts, raining).map(|k| schedule::parse(&s[&k], s, 0)).unwrap_or_default(),
                None => vec![],
            };
        }
        self.update_npc_schedules();
        self.spouse_morning();
        self.children_morning();
        self.pet_morning();
        self.cart_morning();
        self.orders_morning();
    }

    /// End the day (sleep or pass out) and start the next one.
    pub fn end_day(&mut self, passed_out: bool) {
        // Shipping.
        let mut shipped = Vec::new();
        let mut earned = 0;
        let bin: Vec<Item> = self.shipping_bin.drain(..).collect();
        for item in bin {
            if let Some(id) = item.object_id() {
                *self.farmer.shipped.entry(id.to_string()).or_insert(0) += item.stack();
                self.order_shipped(&item);
            }
            let price = self.sell_value(&item) * item.stack();
            earned += price;
            shipped.push((item, price));
        }
        self.farmer.money += earned;
        self.farmer.total_earned += earned as i64;

        let mut money_lost = 0;
        if passed_out && self.current != "FarmHouse" {
            money_lost = (self.farmer.money / 10).min(1000);
            self.farmer.money -= money_lost;
        }

        // Energy for tomorrow (approximation of the documented behaviour:
        // full after a normal night, reduced after late nights/exhaustion).
        let max = self.farmer.max_stamina as f32;
        let bedtime = self.clock.time;
        self.farmer.stamina = if passed_out || self.farmer.exhausted {
            max * 0.5
        } else if bedtime > 2400 {
            let late = crate::time::Clock::minutes_between(2400, bedtime) as f32;
            max * (1.0 - (late / 120.0) * 0.5).clamp(0.5, 1.0)
        } else {
            max
        };
        self.farmer.exhausted = false;
        self.farmer.health = self.farmer.max_health;

        // Friendship upkeep.
        let new_week = self.clock.weekday() == "Sun";
        for (name, f) in self.farmer.friendships.iter_mut() {
            if !f.talked_today && f.points > 0 && self.npcs.iter().any(|n| &n.name == name) {
                f.points = (f.points - 2).max(0);
            }
            f.talked_today = false;
            f.gifted_today = false;
            if new_week {
                f.gifts_this_week = 0;
            }
        }

        // Machines keep working through the night until 6am.
        let overnight = Clock::minutes_between(self.clock.time, 2600) + 240;
        self.advance_machines(overnight);

        let old_season = self.clock.season;
        self.tick_quest_days();
        self.tick_orders();
        self.children_night();
        if let Some((tool, days)) = self.farmer.tool_upgrade.as_mut() {
            *days -= 1;
            if *days <= 0 {
                let name = tool.kind.data_key().to_string();
                self.farmer.mailbox.push(format!("port_tool_ready_{name}"));
            }
        }
        self.clock.next_day();
        self.weather = self.weather_tomorrow;
        let mut rng = self.day_rng(1);
        self.daily_luck = rng.range(-100, 101) as f64 / 1000.0;
        self.weather_tomorrow = roll_weather(&mut rng, self.clock.season, self.clock.day);

        // Overnight world updates.
        let season = self.clock.season;
        let crops = self.items.crops.clone();
        let items = self.items.clone();
        let wild_growth: HashMap<String, f32> = self.wild_trees.iter().map(|(k, d)| (k.clone(), d.growth_chance)).collect();
        let new_season = old_season != season;
        for loc in self.locations.values_mut() {
            let outdoors = loc.outdoors;
            let mut remove = Vec::new();
            for (p, t) in loc.state.terrain.iter_mut() {
                match t {
                    TerrainFeature::HoeDirt(d) => {
                        if let Some(c) = &mut d.crop {
                            if let Some(data) = crops.get(&c.seed_id) {
                                c.new_day(data, d.watered, season, outdoors);
                            }
                        } else if outdoors && d.fertilizer.is_none() && (rng.chance(0.1) || (new_season && season == crate::time::Season::Winter)) {
                            remove.push(*p);
                        }
                        let retention = d.fertilizer.as_ref().and_then(|f| crate::machines::fertilizer_effect(&items, f)).map(|(_, _, r)| r).unwrap_or(0.0);
                        d.watered = d.watered && retention > 0.0 && rng.chance(retention);
                    }
                    TerrainFeature::Tree(tree) => {
                        let chance = wild_growth.get(&tree.kind).copied().unwrap_or(0.2) as f64;
                        if !tree.stump && tree.stage < crate::terrain::TREE_MATURE && rng.chance(chance) {
                            tree.stage += 1;
                        }
                    }
                    TerrainFeature::FruitTree(_) => {}
                    TerrainFeature::Grass(_) => {}
                }
            }
            for p in remove {
                loc.state.terrain.remove(&p);
            }
        }
        self.animals_overnight();
        self.orchard_overnight();
        self.crab_pots_overnight();
        self.house_overnight();
        self.grass_overnight();
        self.check_achievements();
        self.garbage_searched.clear();
        self.shop_sold.clear();
        self.spawn_daily_debris();
        self.leave_mines();
        self.bombs.clear();
        self.explosions.clear();
        self.dropped.clear();
        self.floaters.clear();
        self.weapon_swing = None;

        // Wake up in the farmhouse.
        self.current = "FarmHouse".into();
        let wake = self.wake_tile();
        self.farmer.set_tile(wake);
        self.farmer.facing = Dir::Down;
        self.farmer.tool_use = None;
        self.farmer.tool_charge = None;
        self.fishing = crate::fishing::FishingState::Idle;
        self.in_bed = true;
        self.paused = false;
        self.start_day();
        let report = DayReport { shipped, earned, passed_out, money_lost };
        self.last_report = Some(report.clone());
        self.emit(WorldEvent::DayEnded(report));
    }

    fn spawn_daily_debris(&mut self) {
        let mut rng = self.day_rng(2);
        let items = self.items.clone();
        let Some(farm) = self.locations.get_mut("Farm") else { return };
        if farm.map.prop("SpawnDebris").is_none() && farm.map.width < 60 {
            // Placeholder farms opt in explicitly; large real farms always spawn.
        }
        let (w, h) = (farm.width(), farm.height());
        let count = rng.range(2, 6);
        for _ in 0..count {
            for _attempt in 0..20 {
                let p = TilePos::new(rng.range(0, w), rng.range(0, h));
                if farm.is_clear(p) && farm.is_diggable(p) {
                    let kind = match rng.range(0, 3) {
                        0 => DebrisKind::Stone,
                        1 => DebrisKind::Twig,
                        _ => DebrisKind::Weeds,
                    };
                    farm.state.objects.insert(p, debris_object_from(&items, &mut rng, kind));
                    break;
                }
            }
        }
    }

    // ---------------------------------------------------------------- per tick

    pub fn update(&mut self, input: &PlayerInput) {
        self.update_ms(input, TICK_MS);
    }

    pub fn update_ms(&mut self, input: &PlayerInput, dt: u32) {
        self.game_ms += dt as u64;
        self.animate(dt);
        if self.event.is_some() {
            if self.festival_roaming() {
                if !self.paused {
                    self.update_festival_roam(input, dt);
                }
                return;
            }
            if !self.paused || self.event.as_ref().is_some_and(|e| !e.waiting_ui) {
                self.update_event(dt);
            }
            return;
        }
        if self.paused {
            return;
        }
        for ev in self.clock.advance(dt) {
            match ev {
                ClockEvent::TenMinutes => {
                    self.update_npc_schedules();
                    self.advance_machines(10);
                    self.maybe_lightning();
                    for b in self.farmer.buffs.iter_mut() {
                        b.minutes_left -= 10;
                    }
                    self.farmer.buffs.retain(|b| b.minutes_left > 0);
                    if self.clock.time == 2400 || self.clock.time == 2500 {
                        self.hud("It's getting late...");
                    }
                }
                ClockEvent::PassOut => {
                    self.sound("passOut");
                    self.end_day(true);
                    return;
                }
            }
        }
        self.update_farmer(input, dt);
        self.update_npcs(dt);
        self.update_monsters(dt);
        self.update_projectiles(dt);
        self.update_bombs(dt);
        self.update_animals(dt);
        self.update_pet(dt);
        self.update_children(dt);
        self.update_dropped(dt);
    }

    fn animate(&mut self, _dt: u32) {
        for loc in self.locations.values_mut() {
            for t in loc.state.terrain.values_mut() {
                if let TerrainFeature::Tree(tree) = t {
                    tree.shake = (tree.shake - 1).max(0);
                    if tree.falling > 0 {
                        tree.falling += 1;
                        if tree.falling > 60 {
                            tree.falling = 0;
                        }
                    }
                }
            }
            for o in loc.state.objects.values_mut() {
                o.shake = (o.shake - 1).max(0);
            }
        }
    }

    fn update_farmer(&mut self, input: &PlayerInput, dt: u32) {
        // Toolbar selection.
        if let Some(s) = input.select_slot {
            if s < crate::inventory::TOOLBAR && self.farmer.tool_use.is_none() {
                self.farmer.current_slot = s;
            }
        }
        if input.scroll != 0 && self.farmer.tool_use.is_none() {
            let n = crate::inventory::TOOLBAR as i32;
            let base = (self.farmer.current_slot as i32 / n) * n;
            let cur = self.farmer.current_slot as i32 - base;
            self.farmer.current_slot = (base + (cur + input.scroll).rem_euclid(n)) as usize;
        }

        self.apply_farmer_knockback(dt);

        if self.farmer.freeze_ms > 0 {
            self.farmer.freeze_ms = self.farmer.freeze_ms.saturating_sub(dt);
            self.farmer.moving = false;
            return;
        }

        if self.weapon_swing.is_some() {
            self.advance_weapon_swing(dt);
            self.farmer.moving = false;
            return;
        }

        if self.update_special(dt) || self.update_sling(input.use_held, input.cursor_tile, dt) {
            self.farmer.moving = false;
            return;
        }

        if self.update_tool_charge(input.use_held, dt) {
            self.farmer.moving = false;
            return;
        }

        if self.farmer.tool_use.is_some() {
            self.advance_tool_use(dt);
            self.farmer.moving = false;
            return;
        }

        if self.fishing.active() {
            self.update_fishing(input.use_tool, input.use_held || input.use_tool, dt);
            self.farmer.moving = false;
            return;
        }

        if input.use_tool {
            if let Some(t) = input.cursor_tile {
                self.face_toward(t);
            }
            if self.begin_use(input.cursor_tile) {
                return;
            }
        }
        if input.action {
            if let Some(t) = input.cursor_tile {
                if t.manhattan(self.farmer.tile()) <= 2 {
                    self.face_toward(t);
                }
            }
            if self.do_action(input.cursor_tile) {
                return;
            }
        }

        // Movement (each axis separately so the farmer slides along walls).
        let speed = if input.walk { crate::farmer::WALK_SPEED } else { crate::farmer::RUN_SPEED + self.total_attrs().speed as f32 * 0.5 };
        let mut moved = false;
        if input.move_x != 0 || input.move_y != 0 {
            self.farmer.facing = if input.move_y < 0 {
                Dir::Up
            } else if input.move_y > 0 {
                Dir::Down
            } else if input.move_x > 0 {
                Dir::Right
            } else {
                Dir::Left
            };
            if input.move_x != 0 && input.move_y == 0 {
                self.farmer.facing = if input.move_x > 0 { Dir::Right } else { Dir::Left };
            }
            for (dx, dy) in [(input.move_x as f32 * speed, 0.0), (0.0, input.move_y as f32 * speed)] {
                if dx == 0.0 && dy == 0.0 {
                    continue;
                }
                let np = self.farmer.pos + Vec2::new(dx, dy);
                let bb = Farmer::bbox_at(np);
                if !self.blocked_for_farmer(bb) {
                    self.farmer.pos = np;
                    moved = true;
                } else if let Some(nudged) = self.corner_nudge(np, dx, dy) {
                    self.farmer.pos = nudged;
                    moved = true;
                }
            }
        }
        self.farmer.moving = moved;
        if moved {
            let before = self.farmer.walk_ms;
            self.farmer.walk_ms += dt;
            // A footstep every ~320 ms, sounding like the ground underfoot.
            let step_ms = if input.walk { 450 } else { 320 };
            if before / step_ms != self.farmer.walk_ms / step_ms {
                let ground = self.location().ground_type(self.farmer.tile());
                let has_dirt = matches!(self.location().terrain(self.farmer.tile()), Some(TerrainFeature::HoeDirt(_)));
                let cue = match ground.as_deref() {
                    _ if has_dirt => "sandyStep",
                    Some("Grass") => "grassyStep",
                    Some("Stone") => "stoneStep",
                    Some("Wood") => "woodyStep",
                    Some("Water") => "waterSlosh",
                    _ => "sandyStep",
                };
                self.sound(cue);
            }
        } else {
            self.farmer.walk_ms = 0;
        }

        self.check_warps();
        self.check_bed();
    }

    /// When the farmer clips a corner, slide them around it.
    fn corner_nudge(&self, np: Vec2, dx: f32, dy: f32) -> Option<Vec2> {
        for off in [6.0f32, -6.0, 12.0, -12.0] {
            let candidate = if dx != 0.0 { np + Vec2::new(0.0, off) } else { np + Vec2::new(off, 0.0) };
            if !self.blocked_for_farmer(Farmer::bbox_at(candidate)) {
                // Only nudge a bit per tick.
                let nudge = if dx != 0.0 { Vec2::new(dx, off.signum() * 2.0) } else { Vec2::new(off.signum() * 2.0, dy) };
                let p = self.farmer.pos + nudge;
                if !self.blocked_for_farmer(Farmer::bbox_at(p)) {
                    return Some(p);
                }
            }
        }
        None
    }

    fn blocked_for_farmer(&self, bb: IRect) -> bool {
        let loc = self.location();
        // Allow walking into warp tiles even when they're off-map.
        let warp_hit = loc.warps.iter().any(|w| warp_rect(w.from).intersects(&bb));
        if !warp_hit && box_blocked(loc, bb) {
            return true;
        }
        if warp_hit {
            // Off-map warp tiles: only block on in-bounds blocked tiles.
            let x0 = bb.x.div_euclid(TILE);
            let x1 = (bb.right() - 1).div_euclid(TILE);
            let y0 = bb.y.div_euclid(TILE);
            let y1 = (bb.bottom() - 1).div_euclid(TILE);
            for y in y0..=y1 {
                for x in x0..=x1 {
                    let p = TilePos::new(x, y);
                    if loc.in_bounds(p) && !tile_passable(loc, p) && !loc.warps.iter().any(|w| w.from == p) {
                        return true;
                    }
                }
            }
        }
        self.npcs.iter().any(|n| n.location == self.current && n.bbox().intersects(&shrink(bb, 8)))
    }

    fn check_warps(&mut self) {
        let bb = self.farmer.bbox();
        let feet = TilePos::from_world(bb.center());
        let hit = self.location().warps.iter().find(|w| w.from == feet || (!self.location().in_bounds(w.from) && warp_rect(w.from).intersects(&bb))).cloned();
        if let Some(w) = hit {
            self.warp_farmer(&w.target, w.to, None);
            return;
        }
        if let Some(action) = self.location().touch_action_at(feet) {
            let toks: Vec<&str> = action.split_whitespace().collect();
            if toks.first() == Some(&"Warp") && toks.len() >= 4 {
                if let (Ok(x), Ok(y)) = (toks[2].parse(), toks[3].parse()) {
                    self.warp_farmer(toks[1], TilePos::new(x, y), None);
                }
            }
        }
    }

    fn check_bed(&mut self) {
        if self.current != "FarmHouse" {
            self.in_bed = false;
            return;
        }
        let on_bed = self.location().tile_prop("Back", self.farmer.tile(), "Bed").is_some();
        if on_bed && !self.in_bed {
            self.in_bed = true;
            self.emit(WorldEvent::SleepPrompt);
        } else if !on_bed {
            self.in_bed = false;
        }
    }

    pub fn warp_farmer(&mut self, target: &str, tile: TilePos, facing: Option<Dir>) {
        if !self.locations.contains_key(target) {
            self.hud(format!("{target} isn't available"));
            return;
        }
        let from = std::mem::replace(&mut self.current, target.to_string());
        let loc = self.location();
        // Clamp onto the map and step off the edge so we don't re-trigger warps.
        let mut t = TilePos::new(tile.x.clamp(0, loc.width() - 1), tile.y.clamp(0, loc.height() - 1));
        if loc.warps.iter().any(|w| w.from == t) {
            for d in [Dir::Up, Dir::Down, Dir::Left, Dir::Right] {
                let n = t.offset(d);
                if tile_passable(loc, n) && !loc.warps.iter().any(|w| w.from == n) {
                    t = n;
                    break;
                }
            }
        }
        self.farmer.set_tile(t);
        if let Some(f) = facing {
            self.farmer.facing = f;
        }
        self.in_bed = self.location().tile_prop("Back", t, "Bed").is_some();
        self.sound("doorClose");
        self.quest_visit(target);
        self.emit(WorldEvent::Warped { from, to: target.to_string() });
        if !self.check_festival() {
            self.check_location_events();
        }
    }

    fn face_toward(&mut self, t: TilePos) {
        let me = self.farmer.tile();
        if t != me {
            self.farmer.facing = Dir::toward(t.x - me.x, t.y - me.y);
        }
    }

    /// The tile an action/tool should target: the cursor tile if adjacent,
    /// otherwise the faced tile.
    pub fn target_tile(&self, cursor: Option<TilePos>) -> TilePos {
        let me = self.farmer.tile();
        if let Some(c) = cursor {
            if (c.x - me.x).abs() <= 1 && (c.y - me.y).abs() <= 1 {
                return c;
            }
        }
        self.farmer.facing_tile()
    }

    // ---------------------------------------------------------------- interaction

    /// Right-click / check action. Returns true if something happened.
    pub fn do_action(&mut self, cursor: Option<TilePos>) -> bool {
        let target = self.target_tile(cursor);
        let me = self.farmer.tile();

        // Villagers on or next to the target tile.
        let npc_idx = self.npcs.iter().position(|n| {
            n.location == self.current && (n.tile() == target || (n.tile().manhattan(me) <= 1 && n.tile() != me && cursor.map_or(true, |c| c == n.tile())))
        });
        if let Some(i) = npc_idx {
            let name = self.npcs[i].name.clone();
            if self.try_quest_delivery(&name) || self.try_order_delivery(&name) {
                return true;
            }
            self.interact_npc(i);
            return true;
        }

        // Farm animals, the pet, the traveling cart and buildings.
        if self.pet_animal(target) || self.pet_the_pet(target) || self.interact_child(target) || self.cart_action(target) {
            return true;
        }
        // Trees: tappers and fruit.
        if self.collect_tapper(target) || self.place_tapper(target) || self.shake_fruit_tree(target) {
            return true;
        }
        if self.crab_pot_action(target) || self.place_bomb(target) || self.place_furniture(target) || self.pickup_furniture(target) {
            return true;
        }
        if self.interact_building(target) {
            return true;
        }
        if self.place_hay(target) {
            return true;
        }

        // Harvest / forage.
        if self.try_harvest(target) {
            return true;
        }
        if let Some(PlacedObject { kind: ObjKind::Forage, item_id, .. }) = self.location().object(target).cloned() {
            let botanist = self.farmer.has_profession("botanist") && self.location().outdoors;
            let n = if self.farmer.has_profession("gatherer") && self.rng.chance(0.2) { 2 } else { 1 };
            let item = if botanist { Item::object_q(&item_id, n, crate::item::Quality::Iridium) } else { Item::object(&item_id, n) };
            if self.farmer.inventory.can_fit(&item) {
                self.location_mut().state.objects.remove(&target);
                self.give_item(item);
                self.farmer.skills.gain(Skill::Foraging, 7);
                self.sound("harvest");
            }
            return true;
        }

        // Shipping bin.
        if self.is_shipping_bin(target) {
            return self.ship_current();
        }

        // Placed objects (chests, machines, ladders).
        if self.use_ladder(target) {
            return true;
        }
        if self.interact_object(target) {
            return true;
        }

        // Map actions.
        if let Some(action) = self.location().action_at(target) {
            if self.run_action(&action) {
                return true;
            }
        }
        // Content farms whose farmhouse door isn't a map action.
        if self.current == "Farm" {
            if let Some(door) = self.compat.farmhouse_door {
                if target == door {
                    let entry = self.compat.farmhouse_entry.unwrap_or(TilePos::new(3, 11));
                    self.warp_farmer("FarmHouse", entry, Some(Dir::Up));
                    return true;
                }
            }
        }
        // Farmhouses without a bed tile: the bed spot is where you wake.
        if self.current == "FarmHouse" && self.compat.bed_fallback && target.manhattan(self.wake_tile()) <= 1 && self.farmer.current_item().is_none() {
            self.emit(WorldEvent::SleepPrompt);
            return true;
        }
        if self.current == "FarmHouse" && self.location().action_at(target).is_none() && self.location().tile_prop("Buildings", target, "TV").is_some() {
            self.watch_tv();
            return true;
        }

        // Planting / fertilizing / placing / eating with the held item.
        let slot = self.farmer.current_slot;
        if let Some(item) = self.farmer.current_item().cloned() {
            if self.is_placeable(&item) && self.place_current(target) {
                return true;
            }
        }
        if let Some(Item::Object { id, .. }) = self.farmer.current_item().cloned() {
            if self.fruit_trees.contains_key(&id) || self.wild_trees.values().any(|d| d.seed_item_id.as_deref().map(|s| crate::item::split_qualified(s).1) == Some(id.as_str())) {
                return self.plant_tree(target);
            }
            if self.items.is_seed(&id) {
                return self.plant(target, &id);
            }
            if crate::machines::fertilizer_effect(&self.items, &id).is_some() && matches!(self.location().terrain(target), Some(TerrainFeature::HoeDirt(_))) {
                return self.apply_fertilizer(target);
            }
            if self.items.object(&id).is_some_and(|o| o.is_edible()) {
                self.emit(WorldEvent::EatPrompt { slot });
                return true;
            }
        }
        // Nothing to interact with: a held melee weapon does its special move.
        self.begin_special_move()
    }

    fn is_shipping_bin(&self, t: TilePos) -> bool {
        if self.current != "Farm" {
            return false;
        }
        let b = self.location().map_prop_tile("ShippingBinTile").or(self.compat.shipping_bin);
        match b {
            Some(b) => t == b || t == TilePos::new(b.x + 1, b.y),
            None => false,
        }
    }

    /// Put the held stack into the shipping bin.
    pub fn ship_current(&mut self) -> bool {
        let slot = self.farmer.current_slot;
        let Some(item) = self.farmer.inventory.get(slot).cloned() else { return false };
        if item.tool().is_some() || self.items.sell_price(&item) <= 0 && item.object_id().and_then(|i| self.items.object(i)).map_or(true, |o| o.data.price <= 0) {
            self.hud("Can't ship that.");
            return true;
        }
        self.farmer.inventory.take(slot);
        self.shipping_bin.push(item);
        self.sound("Ship");
        true
    }

    fn run_action(&mut self, action: &str) -> bool {
        let toks: Vec<&str> = action.split_whitespace().collect();
        let Some(&cmd) = toks.first() else { return false };
        match cmd {
            "Warp" | "WarpCommunityCenter" | "Door" if toks.len() >= 4 => {
                let (Ok(x), Ok(y)) = (toks[1].parse(), toks[2].parse()) else { return false };
                self.warp_farmer(toks[3], TilePos::new(x, y), Some(Dir::Up));
                true
            }
            "LockedDoorWarp" if toks.len() >= 6 => {
                let (Ok(x), Ok(y)) = (toks[1].parse(), toks[2].parse()) else { return false };
                let open: i32 = toks[4].parse().unwrap_or(600);
                let close: i32 = toks[5].parse().unwrap_or(2600);
                if self.clock.time >= open && self.clock.time < close {
                    self.warp_farmer(toks[3], TilePos::new(x, y), Some(Dir::Up));
                } else {
                    let msg = format!("Open from {} to {}.", Clock::time_string(open), Clock::time_string(close));
                    self.emit(WorldEvent::Message(msg));
                }
                true
            }
            "OpenShop" if toks.len() >= 2 => {
                self.open_shop(toks[1]);
                true
            }
            "Message" | "MessageOnce" => {
                let text = action[cmd.len()..].trim().trim_matches('"').to_string();
                self.emit(WorldEvent::Message(text));
                true
            }
            "Mailbox" => {
                self.open_mailbox();
                true
            }
            "TV" => {
                self.watch_tv();
                true
            }
            "HayHopper" | "hayHopper" => self.hay_hopper(),
            "SpecialOrders" => self.open_order_board(),
            // Engine convention: Port.Travel <location> <x> <y> <fare> [open-close]
            "Port.Travel" if toks.len() >= 5 => {
                let (Ok(x), Ok(y), Ok(fare)) = (toks[2].parse(), toks[3].parse(), toks[4].parse()) else { return false };
                if let Some((open, close)) = toks.get(5).and_then(|h| h.split_once('-')) {
                    let (open, close): (i32, i32) = (open.parse().unwrap_or(0), close.parse().unwrap_or(2600));
                    if !(open..close).contains(&self.clock.time) {
                        self.emit(WorldEvent::Message(format!("Service runs from {} to {}.", crate::time::Clock::time_string(open), crate::time::Clock::time_string(close))));
                        return true;
                    }
                }
                if !self.locations.contains_key(toks[1]) {
                    return false;
                }
                self.emit(WorldEvent::TravelPrompt { location: toks[1].to_string(), tile: TilePos::new(x, y), fare });
                true
            }
            "Garbage" => {
                let can = toks.get(1).copied().unwrap_or("default").to_string();
                self.search_garbage(&can)
            }
            "Blacksmith" => {
                self.emit(WorldEvent::OpenBlacksmith);
                true
            }
            "Bundles" | "JunimoNote" => {
                self.emit(WorldEvent::OpenBundles);
                true
            }
            "Museum" | "MuseumMenu" | "Gunther" => {
                self.emit(WorldEvent::OpenMuseum);
                true
            }
            "kitchen" | "Kitchen" => {
                self.emit(WorldEvent::OpenCooking);
                true
            }
            "AnimalShop" => {
                let list: Vec<(String, i32)> = self.animals_data.iter().filter(|(_, d)| d.purchase_price >= 0).map(|(k, d)| (k.clone(), d.purchase_price)).collect();
                self.emit(WorldEvent::OpenAnimalShop(list));
                true
            }
            "Carpenter" => {
                let mut list: Vec<String> = self.buildings_data.keys().cloned().collect();
                if self.house_upgrade_cost().is_some() {
                    list.insert(0, crate::world::HOUSE_UPGRADE_ENTRY.to_string());
                }
                self.emit(WorldEvent::OpenCarpenter(list));
                true
            }
            "Billboard" => {
                self.emit(WorldEvent::OpenBillboard);
                true
            }
            "Mine" | "MineLadder" | "Descend" => {
                // Optional start floor, e.g. "Mine 121" for the deep cavern.
                let start = toks.get(1).and_then(|n| n.parse().ok()).unwrap_or(1);
                let me = self.farmer.tile();
                self.mine_entrance = (self.current.clone(), me);
                self.enter_mine_level(start);
                true
            }
            "MineElevator" => {
                let me = self.farmer.tile();
                self.mine_entrance = (self.current.clone(), me);
                self.emit(WorldEvent::ElevatorMenu { deepest: self.farmer.deepest_mine_level });
                true
            }
            _ => false,
        }
    }

    /// Pay the fare and travel (the trip takes half an hour).
    pub fn travel(&mut self, location: &str, tile: TilePos, fare: i32) -> bool {
        if self.farmer.money < fare {
            self.emit(WorldEvent::Message(format!("You need {fare}g for the fare.")));
            return false;
        }
        self.farmer.money -= fare;
        self.clock.time = crate::time::Clock::add_minutes(self.clock.time, TRAVEL_MINUTES).min(2550);
        self.warp_farmer(location, tile, Some(Dir::Down));
        true
    }

    pub fn open_shop(&mut self, id: &str) {
        let Some(data) = self.shops.get(id).cloned() else {
            self.emit(WorldEvent::Message("The shop is closed.".into()));
            return;
        };
        let mut rng = self.day_rng(3);
        let mail = self.farmer.mail_received.clone();
        let seen = self.farmer.events_seen.clone();
        let mut ctx = QueryCtx { clock: &self.clock, weather: self.weather, mail: &mail, events_seen: &seen, location: &self.current, rng: &mut rng };
        let mut stock = shop::build_stock(id, &data, &self.items, &mut ctx);
        self.apply_shop_sold(&mut stock);
        self.emit(WorldEvent::OpenShop(stock));
    }

    fn open_mailbox(&mut self) {
        if self.farmer.mailbox.is_empty() {
            self.emit(WorldEvent::Message("The mailbox is empty.".into()));
            return;
        }
        let id = self.farmer.mailbox.remove(0);
        let raw = self.mail_data.get(&id).cloned().unwrap_or_else(|| match id.as_str() {
            "welcome" => format!(
                "Dear @,^Welcome to {} Farm! The soil is ready, the seeds are at the general store in town, and the shipping bin by the house will sell anything you put in it overnight.^Good luck!",
                self.farmer.farm_name
            ),
            other if other.starts_with("port_tool_ready_") => "@,^Your tool is ready. Come by the forge to pick it up.^- The blacksmith".to_string(),
            other => other.to_string(),
        });
        let letter = crate::mail::parse(&raw, &self.items, &self.farmer.name);
        for item in &letter.items {
            self.give_item(item.clone());
        }
        if letter.money > 0 {
            self.farmer.money += letter.money;
        }
        if let Some(r) = &letter.recipe {
            if self.cooking.contains_key(r) {
                self.farmer.cooking_recipes.entry(r.clone()).or_insert(0);
            } else if self.recipes.contains_key(r) {
                self.farmer.crafting_recipes.entry(r.clone()).or_insert(0);
            }
        }
        self.farmer.mail_received.push(id.clone());
        self.farmer.letters_read.push(id.clone());
        self.emit(WorldEvent::Letter { id, letter });
    }

    /// Buy from a shop, honouring the entry's daily stock limit.
    pub fn buy_from(&mut self, shop_id: &str, entry: &shop::ShopEntry, count: i32) -> bool {
        let key = format!("{shop_id}:{}", entry.item.qualified_id());
        let sold = self.shop_sold.get(&key).copied().unwrap_or(0);
        let count = if entry.limit >= 0 { count.min(entry.limit - sold) } else { count };
        if count <= 0 || !self.buy(entry, count) {
            return false;
        }
        if entry.limit >= 0 {
            *self.shop_sold.entry(key).or_insert(0) += count;
        }
        true
    }

    /// Remove what was already bought today from limited-stock entries.
    pub fn apply_shop_sold(&self, stock: &mut shop::ShopStock) {
        for e in stock.entries.iter_mut().filter(|e| e.limit >= 0) {
            let bought = self.shop_sold.get(&format!("{}:{}", stock.id, e.item.qualified_id())).copied().unwrap_or(0);
            e.stock = (e.limit - bought).max(0);
        }
        stock.entries.retain(|e| e.stock != 0);
    }

    /// Buy `count` of a shop entry. Returns false if unaffordable / no space.
    pub fn buy(&mut self, entry: &shop::ShopEntry, count: i32) -> bool {
        let cost = entry.price * count;
        if cost > self.farmer.money {
            self.sound("cancel");
            return false;
        }
        let mut item = entry.item.clone();
        item.set_stack(count);
        if !self.farmer.inventory.can_fit(&item) {
            self.hud("Inventory full");
            return false;
        }
        self.farmer.money -= cost;
        self.farmer.inventory.add(item);
        self.sound("purchase");
        true
    }

    /// Sell the stack in `slot` to a shop (at its base sell price).
    pub fn sell_slot(&mut self, slot: usize) -> bool {
        let Some(item) = self.farmer.inventory.get(slot).cloned() else { return false };
        if item.tool().is_some() {
            return false;
        }
        let price = self.sell_value(&item) * item.stack();
        self.farmer.inventory.take(slot);
        self.farmer.money += price;
        self.sound("purchaseClick");
        true
    }

    fn interact_npc(&mut self, i: usize) {
        let me = self.farmer.tile();
        let name = self.npcs[i].name.clone();
        if self.maybe_ask_about_child(&name) {
            return;
        }
        let nt = self.npcs[i].tile();
        self.npcs[i].facing = Dir::toward(me.x - nt.x, me.y - nt.y);
        self.npcs[i].pause_ms = 3000;

        // Gift if holding a giftable object and not talked-then-gifted already.
        let held = self.farmer.current_item().cloned();
        let f = self.farmer.friendship(&name).clone();
        if let Some(Item::Object { id, .. }) = &held {
            if self.try_romance_item(i, id) {
                return;
            }
            if f.talked_today && !f.gifted_today {
                self.give_gift(i, id.clone());
                return;
            }
        }
        if f.talked_today {
            return;
        }
        let first = !f.met;
        let raining = self.weather.is_raining();
        let hearts = f.hearts();
        let npc = &self.npcs[i];
        let key = if npc.dialogue.contains_key("__spouse_today") { Some("__spouse_today".to_string()) } else { dialogue::pick_key(&npc.dialogue, &self.clock, hearts, first, raining) };
        let raw = key.and_then(|k| npc.dialogue.get(&k).cloned()).unwrap_or_else(|| "...".to_string());
        let subs = Substitutions { farmer_name: &self.farmer.name, farm_name: &self.farmer.farm_name, male: self.farmer.male };
        let parsed = dialogue::parse_full(&raw, &subs);
        let pages = parsed.pages;
        let question = parsed.question;
        let romanceable = self.npcs[i].can_romance;
        let fr = self.farmer.friendship(&name);
        fr.met = true;
        fr.talked_today = true;
        fr.points = (fr.points + 20).min(fr.max_points(romanceable));
        if !pages.is_empty() {
            self.emit(WorldEvent::Dialogue { npc: Some(name.clone()), pages });
        }
        if let Some(q) = question {
            self.emit(WorldEvent::DialogueQuestion { npc: name.clone(), prompt: q.prompt.clone(), choices: q.choices.iter().map(|c| c.text.clone()).collect() });
            self.pending_question = Some((name, q));
        }
    }

    /// Gift taste: 0 love, 1 like, 2 dislike, 3 hate, 4 neutral.
    pub fn gift_taste(&self, npc: &str, item_id: &str) -> i32 {
        let cat = self.items.object(item_id).map(|o| o.data.category).unwrap_or(0);
        let list_has = |s: &str| s.split_whitespace().any(|t| t == item_id || t.parse::<i32>().ok() == Some(cat) && cat != 0);
        if let Some(entry) = self.gift_tastes.get(npc) {
            let parts: Vec<&str> = entry.split('/').collect();
            // love, like, dislike, hate, neutral: items at odd indices 1,3,5,7,9.
            for (taste, idx) in [(0, 1), (1, 3), (2, 5), (3, 7), (4, 9)] {
                if parts.get(idx).is_some_and(|s| list_has(s)) {
                    return taste;
                }
            }
        }
        for (taste, key) in [(0, "Universal_Love"), (3, "Universal_Hate"), (1, "Universal_Like"), (2, "Universal_Dislike"), (4, "Universal_Neutral")] {
            if self.gift_tastes.get(key).is_some_and(|s| list_has(s)) {
                return taste;
            }
        }
        4
    }

    fn give_gift(&mut self, i: usize, item_id: String) {
        let name = self.npcs[i].name.clone();
        {
            let f = self.farmer.friendship(&name);
            if f.gifts_this_week >= 2 {
                self.emit(WorldEvent::Message(format!("You've already given {name} two gifts this week.")));
                return;
            }
        }
        let taste = self.gift_taste(&name, &item_id);
        let mut points = match taste {
            0 => 80,
            1 => 45,
            2 => -20,
            3 => -40,
            _ => 20,
        };
        let birthday = self.npcs[i]
            .birthday
            .as_ref()
            .is_some_and(|(s, d)| crate::time::Season::parse(s) == Some(self.clock.season) && *d == self.clock.day);
        if birthday {
            points *= 8;
        }
        let slot = self.farmer.current_slot;
        if let Some(given) = self.farmer.inventory.get(slot).cloned() {
            self.order_gift(&name, &given);
        }
        self.farmer.inventory.reduce(slot, 1);
        let romanceable = self.npcs[i].can_romance;
        let f = self.farmer.friendship(&name);
        f.points = (f.points + points).clamp(0, f.max_points(romanceable));
        f.gifted_today = true;
        f.gifts_this_week += 1;
        let (fallback, emo) = match taste {
            0 => ("Oh, I love this! Thank you!", dialogue::Emotion::Happy),
            1 => ("Thanks, this is nice.", dialogue::Emotion::Happy),
            2 => ("Oh... thanks, I guess.", dialogue::Emotion::Sad),
            3 => ("What? Why would you give me this?", dialogue::Emotion::Angry),
            _ => ("Thanks.", dialogue::Emotion::Neutral),
        };
        // NPCGiftTastes stores a reaction line before each item list.
        let text_idx = match taste {
            0 => 0,
            1 => 2,
            2 => 4,
            3 => 6,
            _ => 8,
        };
        let raw = self.gift_tastes.get(&name).and_then(|e| e.split('/').nth(text_idx).map(|s| s.trim().to_string())).filter(|s| !s.is_empty());
        self.sound("give_gift");
        let pages = match raw {
            Some(r) => {
                let subs = Substitutions { farmer_name: &self.farmer.name, farm_name: &self.farmer.farm_name, male: self.farmer.male };
                let mut p = dialogue::parse(&r, &subs);
                if p.iter().all(|x| x.emotion == dialogue::Emotion::Neutral) {
                    for x in p.iter_mut() {
                        x.emotion = emo;
                    }
                }
                p
            }
            None => vec![DialoguePage { text: fallback.into(), emotion: emo }],
        };
        self.emit(WorldEvent::Dialogue { npc: Some(name), pages });
    }

    /// Eat the item in `slot` (after confirmation).
    pub fn eat(&mut self, slot: usize) {
        let Some(Item::Object { id, quality, .. }) = self.farmer.inventory.get(slot).cloned() else { return };
        let Some(info) = self.items.object(&id).cloned() else { return };
        if !info.is_edible() {
            return;
        }
        self.farmer.inventory.reduce(slot, 1);
        let e = info.energy(quality) as f32;
        self.farmer.stamina = (self.farmer.stamina + e).min(self.farmer.max_stamina as f32);
        if self.farmer.stamina > 0.0 {
            self.farmer.exhausted = false;
        }
        self.farmer.health = (self.farmer.health + info.health(quality)).min(self.farmer.max_health);
        if let Some(b) = &info.data.buffs {
            for buff in crate::buffs::parse_buffs(b, &info.display_name) {
                // A new food buff replaces any buff with the same id.
                self.farmer.buffs.retain(|x| x.id != buff.id);
                self.hud(format!("{}: {}", info.display_name, buff.attrs.describe().join(", ")));
                self.farmer.buffs.push(buff);
            }
        }
        self.farmer.freeze_ms = 1000;
        self.sound(if info.data.is_drink { "gulp" } else { "eat" });
    }

    pub fn plant(&mut self, t: TilePos, seed_id: &str) -> bool {
        let Some(data) = self.items.crops.get(seed_id).cloned() else { return false };
        let outdoors = self.location().outdoors;
        let season = self.clock.season;
        let items = self.items.clone();
        let mut rng = self.rng.clone();
        let ok = {
            let loc = self.location_mut();
            match loc.state.terrain.get_mut(&t) {
                Some(TerrainFeature::HoeDirt(d)) if d.crop.is_none() => {
                    if outdoors && !Crop::grows_in(&data, season) {
                        None
                    } else {
                        let mut c = Crop::new(seed_id, &mut rng);
                        if let Some((_, speed, _)) = d.fertilizer.as_ref().and_then(|f| crate::machines::fertilizer_effect(&items, f)) {
                            if speed > 0.0 {
                                c.apply_speed_bonus(&data, speed);
                            }
                        }
                        d.crop = Some(c);
                        Some(true)
                    }
                }
                _ => Some(false),
            }
        };
        self.rng = rng;
        match ok {
            Some(true) => {
                let slot = self.farmer.current_slot;
                self.farmer.inventory.reduce(slot, 1);
                self.sound("dirtyHit");
                true
            }
            None => {
                self.emit(WorldEvent::Message("This crop can't grow in this season.".into()));
                true
            }
            Some(false) => false,
        }
    }

    pub fn try_harvest(&mut self, t: TilePos) -> bool {
        let Some(TerrainFeature::HoeDirt(d)) = self.location().terrain(t) else { return false };
        let Some(crop) = &d.crop else { return false };
        if crop.dead {
            // Clear dead crops by hand.
            self.location_mut().terrain_mut(t).and_then(|f| f.hoe_dirt_mut()).map(|d| d.crop = None);
            self.sound("weed_cut");
            return true;
        }
        let Some(data) = self.items.crops.get(&crop.seed_id).cloned() else { return false };
        if !crop.is_ready(&data) || data.harvest_method.eq_ignore_ascii_case("Scythe") {
            return false;
        }
        self.harvest_at(t, &data)
    }

    pub fn harvest_at(&mut self, t: TilePos, data: &valley_content::data::CropData) -> bool {
        let level = self.farmer.skills.level(Skill::Farming);
        let items = self.items.clone();
        let mut rng = self.rng.clone();
        let result = {
            let loc = self.location_mut();
            let Some(TerrainFeature::HoeDirt(d)) = loc.state.terrain.get_mut(&t) else { return false };
            let fert = d.fertilizer.as_ref().and_then(|f| crate::machines::fertilizer_effect(&items, f)).map(|(q, _, _)| q).unwrap_or(0);
            let Some(crop) = d.crop.as_mut() else { return false };
            let r = crop.harvest(data, &mut rng, level, fert);
            if let Some((_, true)) = &r {
                d.crop = None;
            }
            r
        };
        self.rng = rng;
        let Some((items, _)) = result else { return false };
        let price = self.items.object(&data.harvest_item_id).map(|o| o.data.price).unwrap_or(0);
        let xp = (16.0 * (0.018 * price as f64 + 1.0).ln()).round() as i32;
        if let Some(l) = self.farmer.skills.gain(Skill::Farming, xp) {
            self.emit(WorldEvent::LevelUp(Skill::Farming, l));
        }
        let first = items[0].clone();
        for it in &items {
            self.quest_obtained(it);
        }
        for it in items {
            if let Some(left) = self.farmer.inventory.add(it) {
                let c = t.world_center();
                self.spawn_drop(left, c);
            }
        }
        self.emit(WorldEvent::Pickup(first));
        self.farmer.freeze_ms = 200;
        self.sound("harvest");
        true
    }

    /// Add an item to the inventory; drops it at the farmer if full.
    pub fn give_item(&mut self, item: Item) {
        let shown = item.clone();
        if let Some(left) = self.farmer.inventory.add(item) {
            let c = self.farmer.center();
            self.spawn_drop(left, c);
        } else {
            self.emit(WorldEvent::Pickup(shown));
        }
    }

    pub fn spawn_drop(&mut self, item: Item, at: Vec2) {
        let vx = self.rng.range(-30, 31) as f32 / 10.0;
        let vy = self.rng.range(-20, 21) as f32 / 10.0;
        let loc = self.current.clone();
        self.dropped.push(DroppedItem { location: loc, item, pos: at, vel: Vec2::new(vx, vy), z: 0.0, vz: 6.0, age_ms: 0 });
    }

    fn update_dropped(&mut self, dt: u32) {
        let center = self.farmer.center();
        let magnet = 128.0 + self.total_attrs().magnetic_radius as f32 * 64.0;
        let mut picked = Vec::new();
        for (i, d) in self.dropped.iter_mut().enumerate() {
            if d.location != self.current {
                continue;
            }
            d.age_ms += dt;
            if d.z > 0.0 || d.vz > 0.0 {
                d.z += d.vz;
                d.vz -= 0.6;
                d.pos += d.vel;
                if d.z <= 0.0 {
                    d.z = 0.0;
                    d.vz = if d.vz < -3.0 { -d.vz * 0.4 } else { 0.0 };
                    d.vel = d.vel * 0.5;
                }
                continue;
            }
            if d.age_ms < 300 {
                continue;
            }
            let to = center - d.pos;
            let dist = to.length();
            if dist < magnet {
                if dist < 24.0 {
                    picked.push(i);
                } else {
                    d.pos += to.normalized() * (6.0 + (magnet - dist).max(0.0) / 16.0);
                }
            }
        }
        for i in picked.into_iter().rev() {
            let d = self.dropped[i].clone();
            if self.farmer.inventory.can_fit(&d.item) {
                self.dropped.remove(i);
                self.farmer.inventory.add(d.item.clone());
                self.sound("coin");
                self.emit(WorldEvent::Pickup(d.item));
            }
        }
    }

    // ---------------------------------------------------------------- NPCs

    fn update_npc_schedules(&mut self) {
        let time = self.clock.time;
        for i in 0..self.npcs.len() {
            loop {
                let npc = &self.npcs[i];
                let Some(entry) = npc.schedule.get(npc.next_entry).cloned() else { break };
                if entry.time > time {
                    break;
                }
                self.npcs[i].next_entry += 1;
                self.route_npc(i, &entry.location, entry.tile, entry.facing);
                if let Some(msg) = entry.message {
                    self.npcs[i].bubble = Some((msg, 4000));
                }
            }
        }
    }

    /// Plan a multi-location route for an NPC.
    pub fn route_npc(&mut self, i: usize, location: &str, tile: TilePos, facing: Dir) {
        let start_loc = self.npcs[i].location.clone();
        let start_tile = self.npcs[i].tile();
        let Some(route) = pathfind::location_route(&start_loc, location, &self.location_edges) else {
            // Unreachable: teleport (matches the original's fallback of warping NPCs).
            if self.locations.contains_key(location) {
                self.npcs[i].location = location.to_string();
                self.npcs[i].set_tile(tile);
                self.npcs[i].facing = facing;
            }
            return;
        };
        let mut legs = VecDeque::new();
        let mut cur_tile = start_tile;
        for w in route.windows(2) {
            let (from, to) = (&w[0], &w[1]);
            let loc = &self.locations[from];
            let mut exits: Vec<(TilePos, TilePos)> = loc.warps.iter().filter(|x| &x.target == to).map(|x| (x.from, x.to)).collect();
            for (door, target, dest) in door_warps(loc) {
                if &target == to {
                    exits.push((door, dest));
                }
            }
            exits.sort_by_key(|(f, _)| f.manhattan(cur_tile));
            let mut found = None;
            for (exit, dest) in exits {
                if let Some(path) = pathfind::astar(cur_tile, exit, 6000, |p| tile_passable(loc, p)) {
                    found = Some((path, dest));
                    break;
                }
            }
            let Some((path, dest)) = found else {
                legs.clear();
                break;
            };
            legs.push_back(RouteLeg { location: from.clone(), path: path.into(), warp_to: Some((to.clone(), dest)) });
            cur_tile = dest;
        }
        if route.len() > 1 && legs.is_empty() {
            self.npcs[i].location = location.to_string();
            self.npcs[i].set_tile(tile);
            self.npcs[i].facing = facing;
            return;
        }
        let loc = &self.locations[location];
        let path = pathfind::astar(cur_tile, tile, 6000, |p| tile_passable(loc, p)).unwrap_or_default();
        legs.push_back(RouteLeg { location: location.to_string(), path: path.into(), warp_to: None });
        let npc = &mut self.npcs[i];
        npc.route = legs;
        npc.end_facing = facing;
    }

    fn update_npcs(&mut self, dt: u32) {
        for npc in self.npcs.iter_mut() {
            npc.step(dt);
        }
    }

    pub fn npcs_here(&self) -> impl Iterator<Item = &Npc> {
        self.npcs.iter().filter(move |n| n.location == self.current)
    }

    /// Unit sell value including quality and profession bonuses.
    pub fn sell_value(&self, item: &Item) -> i32 {
        let base = self.items.sell_price(item);
        let cat = item.object_id().and_then(|id| self.items.object(id)).map(|o| o.data.category).unwrap_or(0);
        (base as f32 * crate::professions::price_multiplier(&self.farmer.professions, cat)).floor() as i32
    }

    /// Choose a profession offered for a pending level-up.
    pub fn choose_profession(&mut self, id: &str) {
        if self.farmer.has_profession(id) {
            return;
        }
        self.farmer.professions.push(id.to_string());
        match id {
            "fighter" => {
                self.farmer.max_health += 15;
                self.farmer.health += 15;
            }
            "defender" => {
                self.farmer.max_health += 25;
                self.farmer.health += 25;
            }
            _ => {}
        }
    }

    /// Answer the pending dialogue question with choice `i`.
    pub fn answer_question(&mut self, i: usize) {
        let Some((npc, q)) = self.pending_question.take() else { return };
        let Some(c) = q.choices.get(i).cloned() else { return };
        let romanceable = self.npcs.iter().find(|n| n.name == npc).is_some_and(|n| n.can_romance);
        let f = self.farmer.friendship(&npc);
        f.points = (f.points + c.friendship).clamp(0, f.max_points(romanceable));
        self.child_answer(&c.id);
        if !c.id.is_empty() && !c.id.starts_with("__") && !self.farmer.dialogue_answers.contains(&c.id) {
            self.farmer.dialogue_answers.push(c.id.clone());
        }
        let raw = match &c.reaction {
            dialogue::Reaction::Key(k) => self.npcs.iter().find(|n| n.name == npc).and_then(|n| n.dialogue.get(k).cloned()),
            dialogue::Reaction::Text(t) => Some(t.clone()),
            dialogue::Reaction::None => None,
        };
        if let Some(raw) = raw {
            let subs = Substitutions { farmer_name: &self.farmer.name, farm_name: &self.farmer.farm_name, male: self.farmer.male };
            let pages = dialogue::parse(&raw, &subs);
            self.emit(WorldEvent::Dialogue { npc: Some(npc), pages });
        }
    }

    /// The TV: tomorrow's weather and today's luck.
    pub fn watch_tv(&mut self) {
        let forecast = match self.weather_tomorrow {
            Weather::Sun => "Tomorrow looks clear and sunny.",
            Weather::Rain => "Expect rain tomorrow. Your crops will thank you.",
            Weather::Storm => "A thunderstorm is on the way tomorrow. Stay safe!",
            Weather::Snow => "Snow is in the forecast for tomorrow.",
            Weather::Wind => "It'll be a breezy day tomorrow.",
        };
        let luck = match self.daily_luck {
            l if l > 0.07 => "The spirits are very happy today. Great luck!",
            l if l > 0.02 => "The spirits are in good humor today.",
            l if l >= -0.02 => "The spirits feel neutral today.",
            l if l >= -0.07 => "The spirits are somewhat annoyed today.",
            _ => "The spirits are very displeased today. Be careful!",
        };
        self.sound("tvClick");
        self.emit(WorldEvent::Message(format!("Weather report: {forecast}\n\nFortune corner: {luck}")));
    }

    // ---------------------------------------------------------------- crafting

    pub fn unlock_recipes(&mut self) {
        let names: Vec<String> = self.recipes.iter().filter(|(_, r)| r.auto_unlocked(&self.farmer)).map(|(k, _)| k.clone()).collect();
        for n in names {
            if !self.farmer.crafting_recipes.contains_key(&n) {
                self.farmer.crafting_recipes.insert(n, 0);
            }
        }
    }

    /// Craft a known recipe into the inventory.
    pub fn craft(&mut self, name: &str) -> bool {
        let Some(r) = self.recipes.get(name).cloned() else { return false };
        if !self.farmer.crafting_recipes.contains_key(name) {
            return false;
        }
        let out = r.output();
        if !self.farmer.inventory.can_fit(&out) {
            self.hud("Inventory full");
            return false;
        }
        let items = self.items.clone();
        let Some(item) = r.craft(&items, &mut self.farmer) else {
            self.sound("cancel");
            return false;
        };
        self.farmer.inventory.add(item.clone());
        *self.farmer.crafting_recipes.entry(name.to_string()).or_insert(0) += 1;
        self.sound("crafting");
        self.quest_crafted(&item);
        self.emit(WorldEvent::Pickup(item));
        true
    }

    /// Move a stack between the farmer's inventory and a chest.
    pub fn chest_transfer(&mut self, chest: TilePos, from_player: bool, slot: usize) {
        let loc_name = self.current.clone();
        let Some(loc) = self.locations.get_mut(&loc_name) else { return };
        let Some(crate::objects::PlacedObject { kind: crate::objects::ObjKind::Chest(inv), .. }) = loc.state.objects.get_mut(&chest) else { return };
        if from_player {
            let Some(item) = self.farmer.inventory.take(slot) else { return };
            if let Some(left) = inv.add(item) {
                self.farmer.inventory.slots[slot] = Some(left);
            }
        } else {
            let Some(item) = inv.take(slot) else { return };
            if let Some(left) = self.farmer.inventory.add(item) {
                inv.slots[slot] = Some(left);
            }
        }
        self.sound("pickUpItem");
    }

    // ---------------------------------------------------------------- queries

    pub fn time_string(&self) -> String {
        Clock::time_string(self.clock.time)
    }

    pub fn is_night(&self) -> bool {
        self.clock.time >= 1900
    }

    pub fn crop_data(&self, seed: &str) -> Option<&valley_content::data::CropData> {
        self.items.crops.get(seed)
    }

    pub fn day_ending(&self) -> bool {
        self.clock.time >= DAY_END
    }
}

/// Door-style warps from Buildings "Action" tiles: (door tile, target, dest).
pub fn door_warps(loc: &GameLocation) -> Vec<(TilePos, String, TilePos)> {
    let mut out = Vec::new();
    let Some(layer) = loc.map.layer("Buildings") else { return out };
    for y in 0..layer.height {
        for x in 0..layer.width {
            let p = TilePos::new(x, y);
            let Some(a) = loc.action_at(p) else { continue };
            let t: Vec<&str> = a.split_whitespace().collect();
            if matches!(t.first(), Some(&"Warp") | Some(&"LockedDoorWarp") | Some(&"Door")) && t.len() >= 4 {
                if let (Ok(x2), Ok(y2)) = (t[1].parse(), t[2].parse()) {
                    out.push((p, t[3].to_string(), TilePos::new(x2, y2)));
                }
            }
        }
    }
    out
}

fn warp_rect(t: TilePos) -> IRect {
    IRect::new(t.x * TILE, t.y * TILE, TILE, TILE)
}

fn shrink(r: IRect, by: i32) -> IRect {
    IRect::new(r.x + by, r.y + by, (r.w - by * 2).max(1), (r.h - by * 2).max(1))
}

/// Well-known debris object ids (first available wins), with placeholders last.
fn debris_candidates(kind: DebrisKind) -> &'static [&'static str] {
    match kind {
        DebrisKind::Weeds => &["674", "675", "784", "785", "Dev_Weeds"],
        DebrisKind::Twig => &["294", "295", "Dev_Twig"],
        DebrisKind::Stone => &["343", "450", "Dev_Rock"],
    }
}

fn debris_object_from(items: &ItemDb, rng: &mut Rng, kind: DebrisKind) -> PlacedObject {
    let avail: Vec<&str> = debris_candidates(kind).iter().copied().filter(|id| items.objects.contains_key(*id)).collect();
    let id = rng.pick(&avail).copied().unwrap_or(debris_candidates(kind).last().unwrap());
    let mut o = PlacedObject::debris(id, kind);
    o.flip = rng.chance(0.5);
    o
}

/// Weather roll for tomorrow (approximate seasonal chances; fixed sunny
/// days at the start of each season, like the original's calendar).
fn roll_weather(rng: &mut Rng, season: crate::time::Season, day: i32) -> Weather {
    use crate::time::Season::*;
    if day == 1 || day == 2 {
        return Weather::Sun;
    }
    let r = rng.next_f64();
    match season {
        Spring if r < 0.183 => Weather::Rain,
        Spring if r < 0.25 => Weather::Wind,
        Summer if r < 0.12 => Weather::Storm,
        Summer if r < 0.18 => Weather::Rain,
        Fall if r < 0.183 => Weather::Rain,
        Fall if r < 0.3 => Weather::Wind,
        Winter if r < 0.4 => Weather::Snow,
        _ => Weather::Sun,
    }
}
