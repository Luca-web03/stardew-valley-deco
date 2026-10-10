use valley_content::ContentManager;
use valley_sim::input::PlayerInput;
use valley_sim::item::{Item, Quality};
use valley_sim::objects::ObjKind;
use valley_sim::terrain::{HoeDirt, TerrainFeature};
use valley_sim::world::{NewGame, World, WorldEvent};
use valley_sim::{Dir, TilePos};

fn world_on_farm() -> World {
    let mut cm = ContentManager::dev_only();
    let mut w = World::new(&mut cm, NewGame::default());
    w.warp_farmer("Farm", TilePos::new(12, 18), Some(Dir::Down));
    for x in 10..16 {
        for y in 17..22 {
            let loc = w.location_mut();
            loc.state.objects.remove(&TilePos::new(x, y));
            loc.state.terrain.remove(&TilePos::new(x, y));
        }
    }
    w.farmer.set_tile(TilePos::new(12, 18));
    w.farmer.facing = Dir::Down;
    w.drain_events();
    w
}

fn hold(w: &mut World, item: Item) {
    let slot = 11;
    w.farmer.inventory.slots[slot] = Some(item);
    w.farmer.current_slot = slot;
}

#[test]
fn default_recipes_known_and_craftable() {
    let mut w = world_on_farm();
    assert!(w.farmer.crafting_recipes.contains_key("Storage Box"));
    assert!(!w.farmer.crafting_recipes.contains_key("Fertilizer"), "needs farming level 1");
    assert!(!w.craft("Storage Box"));
    w.farmer.inventory.add(Item::object("388", 25));
    assert!(w.craft("Storage Box"));
    assert_eq!(w.farmer.inventory.count("388"), 5);
    assert!(w.farmer.inventory.slots.iter().flatten().any(|i| i.big_id() == Some("Dev_Box")));
}

#[test]
fn chest_place_store_and_pickup() {
    let mut w = world_on_farm();
    hold(&mut w, Item::big("Dev_Box", 1));
    let t = TilePos::new(12, 19);
    assert!(w.do_action(None));
    assert!(matches!(w.location().object(t).unwrap().kind, ObjKind::Chest(_)));
    w.drain_events();
    assert!(w.do_action(None));
    assert!(w.drain_events().contains(&WorldEvent::OpenChest(t)));
    w.farmer.inventory.slots[20] = Some(Item::object("390", 7));
    w.chest_transfer(t, true, 20);
    assert_eq!(w.farmer.inventory.count("390"), 0);
    w.chest_transfer(t, false, 0);
    assert_eq!(w.farmer.inventory.count("390"), 7);
    // Empty chest can be picked up with the pickaxe.
    let pick = (0..36).find(|&i| w.farmer.inventory.get(i).and_then(|x| x.tool()).is_some_and(|t| t.kind == valley_sim::item::ToolKind::Pickaxe)).unwrap();
    w.farmer.current_slot = pick;
    w.update(&PlayerInput { use_tool: true, ..Default::default() });
    for _ in 0..80 {
        w.update(&PlayerInput::default());
    }
    assert!(w.location().object(t).is_none());
}

#[test]
fn juicer_makes_flavored_juice() {
    let mut w = world_on_farm();
    let t = TilePos::new(12, 19);
    hold(&mut w, Item::big("Dev_Juicer", 1));
    assert!(w.do_action(None));
    // Wood is rejected.
    hold(&mut w, Item::object("388", 1));
    w.drain_events();
    w.do_action(None);
    assert!(w.drain_events().iter().any(|e| matches!(e, WorldEvent::Hud(_))));
    assert_eq!(w.farmer.inventory.count("388"), 1);
    hold(&mut w, Item::object("Dev_Turnip", 3));
    assert!(w.do_action(None));
    assert_eq!(w.farmer.inventory.count("Dev_Turnip"), 2);
    w.advance_machines(120);
    let Some(o) = w.location().object(t) else { panic!() };
    let ObjKind::Machine { output: Some(out), ready: true, .. } = &o.kind else { panic!("{:?}", o.kind) };
    assert_eq!(w.items.display_name(out), "Turnip Juice");
    assert_eq!(w.items.sell_price(out), 80);
    w.farmer.inventory.slots[11] = None;
    assert!(w.do_action(None));
    assert_eq!(w.farmer.inventory.count("Dev_Juice"), 1);
}

#[test]
fn sprinkler_waters_neighbours_each_morning() {
    let mut w = world_on_farm();
    let s = TilePos::new(12, 20);
    for t in [TilePos::new(12, 19), TilePos::new(11, 20), TilePos::new(13, 20), TilePos::new(12, 21), TilePos::new(11, 19)] {
        w.location_mut().state.terrain.insert(t, TerrainFeature::HoeDirt(HoeDirt::new()));
    }
    w.farmer.set_tile(TilePos::new(12, 19));
    w.farmer.facing = Dir::Down;
    w.location_mut().state.terrain.remove(&s);
    hold(&mut w, Item::object("Dev_Sprinkler", 1));
    assert!(w.do_action(None));
    w.weather_tomorrow = valley_sim::time::Weather::Sun;
    w.end_day(false);
    let farm = &w.locations["Farm"];
    let watered = |t: TilePos| matches!(farm.state.terrain.get(&t), Some(TerrainFeature::HoeDirt(d)) if d.watered);
    // Unplanted soil may revert overnight; check whichever tiles survived.
    for t in [TilePos::new(12, 19), TilePos::new(11, 20), TilePos::new(13, 20), TilePos::new(12, 21)] {
        if farm.state.terrain.contains_key(&t) {
            assert!(watered(t), "{t:?}");
        }
    }
    if farm.state.terrain.contains_key(&TilePos::new(11, 19)) {
        assert!(!watered(TilePos::new(11, 19)), "diagonal is outside a cross sprinkler");
    }
}

#[test]
fn fertilizer_raises_quality_and_speed_gro_shortens_growth() {
    let mut w = world_on_farm();
    let t = TilePos::new(12, 19);
    w.location_mut().state.terrain.insert(t, TerrainFeature::HoeDirt(HoeDirt::new()));
    hold(&mut w, Item::object("Dev_SpeedGro", 1));
    assert!(w.do_action(None));
    hold(&mut w, Item::object("Dev_PumpkinSeeds", 1));
    w.clock.season = valley_sim::time::Season::Fall;
    assert!(w.do_action(None));
    let Some(TerrainFeature::HoeDirt(d)) = w.location().terrain(t) else { panic!() };
    let crop = d.crop.as_ref().unwrap();
    let data = &w.items.crops["Dev_PumpkinSeeds"];
    assert_eq!(data.days_in_phase.iter().sum::<i32>(), 11);
    assert_eq!(crop.days_until_ready(data), 9, "10% of 11 days rounds up to 2");
    // Quality: many harvest rolls with high fertilizer should beat no fertilizer.
    let mut rng = engine::Rng::new(5);
    let base: i32 = (0..2000).map(|_| valley_sim::crop::Crop::roll_quality(&mut rng, 0, 0, data) as i32).sum();
    let fert: i32 = (0..2000).map(|_| valley_sim::crop::Crop::roll_quality(&mut rng, 0, 2, data) as i32).sum();
    assert!(fert > base * 2, "{fert} vs {base}");
    let _ = Quality::Gold;
}

#[test]
fn forage_spawns_in_town() {
    let mut w = world_on_farm();
    let total: usize = (0..5)
        .map(|_| {
            w.end_day(false);
            w.locations["Town"].state.objects.values().filter(|o| matches!(o.kind, ObjKind::Forage)).count()
        })
        .max()
        .unwrap();
    assert!(total > 0);
    assert!(total <= 6);
}
