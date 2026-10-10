use valley_content::ContentManager;
use valley_sim::farmer::Skill;
use valley_sim::input::PlayerInput;
use valley_sim::item::{Item, Tool, ToolKind};
use valley_sim::objects::ObjKind;
use valley_sim::terrain::{HoeDirt, TerrainFeature};
use valley_sim::world::{NewGame, World, WorldEvent};
use valley_sim::{Dir, TilePos};

fn world() -> World {
    let mut cm = ContentManager::dev_only();
    World::new(&mut cm, NewGame::default())
}

#[test]
fn level_ups_are_queued_and_professions_change_prices() {
    let mut w = world();
    w.drain_events();
    let gained = w.farmer.skills.gain(Skill::Farming, 800);
    assert_eq!(gained, Some(3));
    w.emit(WorldEvent::LevelUp(Skill::Farming, 3));
    assert_eq!(w.farmer.pending_levelups, vec![(Skill::Farming, 3)]);
    let turnip = Item::object("Dev_Turnip", 1);
    let before = w.sell_value(&turnip);
    w.choose_profession("tiller");
    assert_eq!(w.sell_value(&turnip), (before as f32 * 1.1).floor() as i32);
    w.choose_profession("fighter");
    assert_eq!(w.farmer.max_health, 115);
}

#[test]
fn tool_upgrade_takes_two_days() {
    let mut w = world();
    let hoe = (0..36).find(|&i| w.farmer.inventory.get(i).and_then(|x| x.tool()).is_some_and(|t| t.kind == ToolKind::Hoe)).unwrap();
    assert!(w.start_tool_upgrade(hoe).is_err(), "needs money and bars");
    w.farmer.money = 5000;
    w.farmer.inventory.add(Item::object("Dev_CopperBar", 5));
    assert!(w.start_tool_upgrade(hoe).is_ok());
    assert_eq!(w.farmer.money, 3000);
    assert!(w.farmer.inventory.get(hoe).is_none());
    assert!(!w.collect_tool_upgrade());
    w.end_day(false);
    assert!(!w.collect_tool_upgrade());
    w.end_day(false);
    assert!(w.collect_tool_upgrade());
    let lvl = w.farmer.inventory.slots.iter().flatten().filter_map(|i| i.tool()).find(|t| t.kind == ToolKind::Hoe).unwrap().level;
    assert_eq!(lvl, 1);
}

#[test]
fn charged_watering_can_covers_a_line() {
    let mut w = world();
    w.warp_farmer("Farm", TilePos::new(12, 17), Some(Dir::Down));
    w.farmer.set_tile(TilePos::new(12, 17));
    w.farmer.facing = Dir::Down;
    for y in 18..21 {
        let t = TilePos::new(12, y);
        w.location_mut().state.objects.remove(&t);
        w.location_mut().state.terrain.insert(t, TerrainFeature::HoeDirt(HoeDirt::new()));
    }
    let mut can = Tool::new(ToolKind::WateringCan);
    can.level = 1;
    can.water = can.water_capacity();
    w.farmer.inventory.slots[11] = Some(Item::Tool(can));
    w.farmer.current_slot = 11;
    w.update(&PlayerInput { use_tool: true, use_held: true, ..Default::default() });
    assert!(w.farmer.tool_charge.is_some());
    for _ in 0..45 {
        w.update(&PlayerInput { use_held: true, ..Default::default() });
    }
    assert_eq!(w.farmer.tool_charge.as_ref().unwrap().power(), 1);
    w.update(&PlayerInput::default());
    for _ in 0..40 {
        w.update(&PlayerInput::default());
    }
    for y in 18..21 {
        let Some(TerrainFeature::HoeDirt(d)) = w.location().terrain(TilePos::new(12, y)) else { panic!() };
        assert!(d.watered, "row {y}");
    }
}

#[test]
fn furnace_smelts_with_coal_and_geodes_crack() {
    let mut w = world();
    w.warp_farmer("Farm", TilePos::new(12, 17), Some(Dir::Down));
    w.farmer.set_tile(TilePos::new(12, 17));
    w.farmer.facing = Dir::Down;
    let t = TilePos::new(12, 18);
    w.location_mut().state.objects.remove(&t);
    w.location_mut().state.terrain.remove(&t);
    w.farmer.inventory.slots[11] = Some(Item::big("Dev_Furnace", 1));
    w.farmer.current_slot = 11;
    assert!(w.do_action(None));
    w.farmer.inventory.slots[11] = Some(Item::object("Dev_CopperOre", 6));
    w.drain_events();
    w.do_action(None);
    assert_eq!(w.farmer.inventory.count("Dev_CopperOre"), 6, "no coal, nothing happens");
    w.farmer.inventory.add(Item::object("Dev_Coal", 1));
    w.do_action(None);
    assert_eq!(w.farmer.inventory.count("Dev_CopperOre"), 1);
    assert_eq!(w.farmer.inventory.count("Dev_Coal"), 0);
    w.advance_machines(30);
    assert!(matches!(w.location().object(t).unwrap().kind, ObjKind::Machine { ready: true, .. }));
    w.farmer.money = 100;
    w.farmer.inventory.slots[10] = Some(Item::object("Dev_Geode", 2));
    let found = w.crack_geode(10).unwrap();
    assert!(w.items.object(found.object_id().unwrap()).is_some());
    assert_eq!(w.farmer.money, 75);
    assert_eq!(w.farmer.inventory.count("Dev_Geode"), 1);
}
