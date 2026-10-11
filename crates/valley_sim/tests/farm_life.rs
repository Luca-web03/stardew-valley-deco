use valley_content::ContentManager;
use valley_sim::clumps::{ClumpKind, ResourceClump};
use valley_sim::crop::Crop;
use valley_sim::farm_events::NightEvent;
use valley_sim::item::{Item, Tool, ToolKind};
use valley_sim::objects::{ObjKind, PlacedObject};
use valley_sim::terrain::{HoeDirt, TerrainFeature};
use valley_sim::time::{Season, Weather};
use valley_sim::world::{NewGame, World, WorldEvent};
use valley_sim::{Dir, TilePos};

fn world() -> World {
    let mut cm = ContentManager::dev_only();
    World::new(&mut cm, NewGame::default())
}

fn clear_farm(w: &mut World, x0: i32, y0: i32, x1: i32, y1: i32) {
    let farm = w.locations.get_mut("Farm").unwrap();
    for y in y0..y1 {
        for x in x0..x1 {
            farm.state.objects.remove(&TilePos::new(x, y));
            farm.state.terrain.remove(&TilePos::new(x, y));
        }
    }
    farm.state.clumps.retain(|c| c.tiles().iter().all(|t| t.x < x0 || t.x >= x1 || t.y < y0 || t.y >= y1));
}

fn plant(w: &mut World, seed: &str, t: TilePos, ready: bool) {
    let data = w.items.crops[seed].clone();
    let mut crop = Crop::new(seed, &mut engine::Rng::new(1));
    if ready {
        crop.phase = data.days_in_phase.len();
    }
    let mut dirt = HoeDirt::default();
    dirt.crop = Some(crop);
    dirt.watered = true;
    w.locations.get_mut("Farm").unwrap().state.terrain.insert(t, TerrainFeature::HoeDirt(dirt));
}

fn give_tool(w: &mut World, kind: ToolKind, level: i32) -> usize {
    if w.current != "Farm" {
        w.warp_farmer("Farm", TilePos::new(21, 21), Some(Dir::Down));
    }
    let mut t = Tool::new(kind);
    t.level = level;
    w.farmer.inventory.slots[0] = Some(Item::Tool(t));
    0
}

fn crops_left(w: &World) -> usize {
    w.locations["Farm"].state.terrain.values().filter(|t| matches!(t, TerrainFeature::HoeDirt(d) if d.crop.is_some())).count()
}

#[test]
fn new_farm_has_clumps_that_need_upgraded_tools() {
    let mut w = world();
    let farm = &w.locations["Farm"];
    assert!(!farm.state.clumps.is_empty(), "a new farm has stumps and boulders");
    // Clumps block movement.
    let c = farm.state.clumps[0].clone();
    assert!(!valley_sim::collision::tile_passable(farm, c.tile));

    clear_farm(&mut w, 20, 20, 30, 30);
    let stump = TilePos::new(22, 22);
    w.locations.get_mut("Farm").unwrap().state.clumps.push(ResourceClump::new(ClumpKind::Stump, stump));
    let slot = give_tool(&mut w, ToolKind::Axe, 0);
    w.drain_events();
    w.apply_tool(ToolKind::Axe, slot, TilePos::new(23, 23));
    assert!(w.drain_events().iter().any(|e| matches!(e, WorldEvent::Message(m) if m.contains("too tough"))));
    assert_eq!(w.locations["Farm"].state.clumps.last().unwrap().health, ClumpKind::Stump.base_health());

    give_tool(&mut w, ToolKind::Axe, 1);
    let mut hits = 0;
    while w.locations["Farm"].clump_at(stump).is_some() {
        w.apply_tool(ToolKind::Axe, slot, stump);
        hits += 1;
        assert!(hits < 20);
    }
    assert!(hits > 1, "takes several hits ({hits})");
    let hardwood: i32 = w.dropped.iter().filter(|d| d.item.object_id() == Some("709")).map(|d| d.item.stack()).sum();
    assert_eq!(hardwood, 2);
    assert!(valley_sim::collision::tile_passable(&w.locations["Farm"], stump));

    // A boulder needs a pickaxe, not an axe.
    let rock = TilePos::new(26, 26);
    w.locations.get_mut("Farm").unwrap().state.clumps.push(ResourceClump::new(ClumpKind::Boulder, rock));
    give_tool(&mut w, ToolKind::Axe, 4);
    w.apply_tool(ToolKind::Axe, slot, rock);
    assert!(w.locations["Farm"].clump_at(rock).is_some());
    give_tool(&mut w, ToolKind::Pickaxe, 4);
    for _ in 0..5 {
        w.apply_tool(ToolKind::Pickaxe, slot, rock);
    }
    assert!(w.locations["Farm"].clump_at(rock).is_none());
}

#[test]
fn ripe_squares_can_turn_into_giant_crops() {
    let mut w = world();
    w.giant_crops.get_mut("Dev_Pumpkin").unwrap().chance = 1.0;
    w.clock.season = Season::Fall;
    w.weather_tomorrow = Weather::Sun;
    clear_farm(&mut w, 20, 20, 30, 30);
    for y in 22..25 {
        for x in 22..25 {
            plant(&mut w, "Dev_PumpkinSeeds", TilePos::new(x, y), true);
        }
    }
    // A lone ripe pumpkin stays a pumpkin.
    plant(&mut w, "Dev_PumpkinSeeds", TilePos::new(28, 28), true);
    w.end_day(false);
    let farm = &w.locations["Farm"];
    let giant = farm.state.clumps.iter().find(|c| matches!(&c.kind, ClumpKind::GiantCrop(k) if k == "Dev_Pumpkin")).cloned().expect("giant pumpkin");
    assert_eq!((giant.tile, giant.size), (TilePos::new(22, 22), (3, 3)));
    assert!(matches!(farm.terrain(TilePos::new(28, 28)), Some(TerrainFeature::HoeDirt(d)) if d.crop.is_some()));
    assert!(matches!(farm.terrain(TilePos::new(23, 23)), Some(TerrainFeature::HoeDirt(d)) if d.crop.is_none()));

    let slot = give_tool(&mut w, ToolKind::Axe, 0);
    for _ in 0..3 {
        w.apply_tool(ToolKind::Axe, slot, TilePos::new(23, 24));
    }
    assert!(w.locations["Farm"].clump_at(TilePos::new(23, 23)).is_none());
    let pumpkins: i32 = w.dropped.iter().filter(|d| d.item.object_id() == Some("Dev_Pumpkin")).map(|d| d.item.stack()).sum();
    assert!((15..=21).contains(&pumpkins), "{pumpkins} pumpkins");
}

#[test]
fn crows_eat_unprotected_crops_and_scarecrows_stop_them() {
    let setup = |scarecrow: bool| {
        let mut w = world();
        w.clock.season = Season::Spring;
        clear_farm(&mut w, 10, 10, 40, 40);
        for y in 20..24 {
            for x in 20..36 {
                plant(&mut w, "Dev_TurnipSeeds", TilePos::new(x, y), false);
            }
        }
        if scarecrow {
            w.locations.get_mut("Farm").unwrap().state.objects.insert(TilePos::new(28, 24), PlacedObject::new("Dev_Scarecrow", true, ObjKind::Placed));
        }
        w
    };
    let mut w = setup(false);
    let start = crops_left(&w);
    assert_eq!(start, 64);
    let mut saw_crow = false;
    for _ in 0..8 {
        w.weather_tomorrow = Weather::Rain;
        w.end_day(false);
        saw_crow |= !w.crows.is_empty();
    }
    assert!(crops_left(&w) < start, "crows ate something over 8 nights");
    assert!(saw_crow);

    let mut w = setup(true);
    assert!(w.scarecrow_covers("Farm", TilePos::new(20, 20)));
    for _ in 0..8 {
        w.weather_tomorrow = Weather::Rain;
        w.end_day(false);
    }
    assert_eq!(crops_left(&w), 64, "scarecrow protects every crop");
}

#[test]
fn crows_fly_off_when_approached() {
    let mut w = world();
    w.crows.push(valley_sim::farm_events::Crow { location: "Farm".into(), pos: TilePos::new(30, 30).world_center(), flying: false, ms: 0, flip: false });
    w.warp_farmer("Farm", TilePos::new(30, 32), Some(Dir::Up));
    w.update_ms(&Default::default(), 16);
    assert!(w.crows[0].flying);
    for _ in 0..300 {
        w.update_ms(&Default::default(), 16);
    }
    assert!(w.crows.is_empty());
}

#[test]
fn earthquake_comes_on_the_third_of_summer() {
    let mut w = world();
    w.clock.season = Season::Summer;
    w.clock.day = 2;
    w.end_day(false);
    assert_eq!(w.last_report.as_ref().unwrap().night_event, Some(NightEvent::Earthquake));
    assert!(w.farmer.mail_received.iter().any(|m| m == "Port_Earthquake"));
}

#[test]
fn night_events_happen_now_and_then() {
    let mut w = world();
    clear_farm(&mut w, 10, 10, 40, 40);
    let mut seen = Vec::new();
    for _ in 0..400 {
        w.end_day(false);
        if let Some(e) = w.last_report.as_ref().unwrap().night_event {
            seen.push(e);
        }
    }
    assert!(!seen.is_empty(), "some night event in 400 nights");
    if seen.contains(&NightEvent::Meteorite) {
        assert!(w.locations["Farm"].state.clumps.iter().any(|c| c.kind == ClumpKind::Meteorite));
    }
}

#[test]
fn rain_totem_makes_tomorrow_rainy() {
    let mut w = world();
    w.weather_tomorrow = Weather::Sun;
    w.farmer.inventory.slots[3] = Some(Item::object("Dev_RainTotem", 2));
    w.farmer.current_slot = 3;
    w.warp_farmer("Farm", TilePos::new(30, 30), Some(Dir::Down));
    clear_farm(&mut w, 28, 28, 33, 33);
    assert!(w.do_action(Some(TilePos::new(30, 31))));
    assert_eq!(w.weather_tomorrow, Weather::Rain);
    assert_eq!(w.farmer.inventory.count("Dev_RainTotem"), 1);
    w.end_day(false);
    assert_eq!(w.weather, Weather::Rain);
}

#[test]
fn green_rain_comes_once_each_summer() {
    let mut w = world();
    let day = w.green_rain_day(1);
    assert!(valley_sim::time::GREEN_RAIN_DAYS.contains(&day));
    w.clock.season = Season::Summer;
    w.clock.day = day - 2;
    w.end_day(false);
    assert_eq!(w.weather_tomorrow, Weather::GreenRain, "forecast the day before");
    clear_farm(&mut w, 10, 10, 40, 34);
    w.end_day(false);
    assert_eq!(w.weather, Weather::GreenRain);
    assert!(w.weather.is_raining(), "it waters the crops");
    let weeds = w.locations["Farm"].state.objects.iter().filter(|(p, o)| (10..40).contains(&p.x) && (10..34).contains(&p.y) && matches!(o.kind, ObjKind::Debris { .. })).count();
    assert!(weeds > 5, "mossy weeds sprouted ({weeds})");
    w.end_day(false);
    assert_ne!(w.weather, Weather::GreenRain);
}
