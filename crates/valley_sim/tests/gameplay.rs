use valley_content::ContentManager;
use valley_sim::input::PlayerInput;
use valley_sim::item::{Item, ToolKind};
use valley_sim::save::SaveData;
use valley_sim::terrain::TerrainFeature;
use valley_sim::world::{NewGame, World, WorldEvent};
use valley_sim::{Dir, TilePos};

fn new_world() -> (ContentManager, World) {
    let mut cm = ContentManager::dev_only();
    let w = World::new(&mut cm, NewGame::default());
    (cm, w)
}

fn select_tool(w: &mut World, kind: ToolKind) {
    let slot = (0..w.farmer.inventory.capacity())
        .find(|&i| w.farmer.inventory.get(i).and_then(|it| it.tool()).is_some_and(|t| t.kind == kind))
        .expect("tool in inventory");
    w.farmer.current_slot = slot;
}

fn swing(w: &mut World) {
    w.update(&PlayerInput { use_tool: true, ..Default::default() });
    for _ in 0..40 {
        w.update(&PlayerInput::default());
    }
}

fn clear_tile(w: &mut World, t: TilePos) {
    let loc = w.location_mut();
    loc.state.objects.remove(&t);
    loc.state.terrain.remove(&t);
}

#[test]
fn starts_in_farmhouse_morning() {
    let (_, w) = new_world();
    assert_eq!(w.current, "FarmHouse");
    assert_eq!(w.clock.time, 600);
    assert_eq!(w.farmer.money, 500);
    assert!(!w.npcs.is_empty());
    assert!(w.location().has_tile("Back", w.farmer.tile()));
}

#[test]
fn walking_moves_and_collides() {
    let (_, mut w) = new_world();
    let start = w.farmer.pos;
    for _ in 0..10 {
        w.update(&PlayerInput { move_x: -1, ..Default::default() });
    }
    assert!(w.farmer.pos.x < start.x);
    assert_eq!(w.farmer.facing, Dir::Left);
    // Walk into the west wall for a long time: must stay inside the map.
    for _ in 0..400 {
        w.update(&PlayerInput { move_x: -1, ..Default::default() });
    }
    assert!(w.farmer.tile().x >= 1, "{:?}", w.farmer.tile());
}

#[test]
fn leaves_house_through_door_warp() {
    let (_, mut w) = new_world();
    w.farmer.set_tile(TilePos::new(5, 9));
    for _ in 0..60 {
        w.update(&PlayerInput { move_y: 1, ..Default::default() });
        if w.current == "Farm" {
            break;
        }
    }
    assert_eq!(w.current, "Farm");
    assert!(w.drain_events().iter().any(|e| matches!(e, WorldEvent::Warped { .. })));
}

#[test]
fn full_crop_cycle_and_shipping() {
    let (_, mut w) = new_world();
    w.warp_farmer("Farm", TilePos::new(12, 18), Some(Dir::Down));
    let target = TilePos::new(12, 19);
    clear_tile(&mut w, target);
    w.farmer.facing = Dir::Down;

    select_tool(&mut w, ToolKind::Hoe);
    let stamina = w.farmer.stamina;
    swing(&mut w);
    assert!(matches!(w.location().terrain(target), Some(TerrainFeature::HoeDirt(_))));
    assert!(w.farmer.stamina < stamina);

    w.farmer.inventory.add(Item::object("Dev_TurnipSeeds", 5));
    let seed_slot = (0..36).find(|&i| w.farmer.inventory.get(i).and_then(|x| x.object_id()) == Some("Dev_TurnipSeeds")).unwrap();
    w.farmer.current_slot = seed_slot;
    assert!(w.do_action(None));
    assert_eq!(w.farmer.inventory.count("Dev_TurnipSeeds"), 4);

    for day in 0..4 {
        select_tool(&mut w, ToolKind::WateringCan);
        swing(&mut w);
        let Some(TerrainFeature::HoeDirt(d)) = w.location().terrain(target) else { panic!("dirt gone day {day}") };
        assert!(d.watered, "watered day {day}");
        w.end_day(false);
        w.warp_farmer("Farm", TilePos::new(12, 18), Some(Dir::Down));
    }
    assert_eq!(w.clock.day, 5);
    let Some(TerrainFeature::HoeDirt(d)) = w.location().terrain(target) else { panic!() };
    let data = w.items.crops["Dev_TurnipSeeds"].clone();
    assert!(d.crop.as_ref().unwrap().is_ready(&data));

    w.farmer.current_slot = 11;
    assert!(w.do_action(None));
    assert_eq!(w.farmer.inventory.count("Dev_Turnip"), 1);
    assert!(w.farmer.skills.xp(valley_sim::farmer::Skill::Farming) > 0);

    // Ship it.
    let slot = (0..36).find(|&i| w.farmer.inventory.get(i).and_then(|x| x.object_id()) == Some("Dev_Turnip")).unwrap();
    w.farmer.current_slot = slot;
    let bin = w.location().map_prop_tile("ShippingBinTile").unwrap();
    w.farmer.set_tile(TilePos::new(bin.x, bin.y + 1));
    w.farmer.facing = Dir::Up;
    assert!(w.do_action(None));
    assert_eq!(w.shipping_bin.len(), 1);
    let money = w.farmer.money;
    w.end_day(false);
    assert!(w.farmer.money > money);
}

#[test]
fn trees_fall_and_drop_wood() {
    let (_, mut w) = new_world();
    w.warp_farmer("Farm", TilePos::new(12, 18), Some(Dir::Right));
    let t = TilePos::new(13, 18);
    clear_tile(&mut w, t);
    w.location_mut().state.terrain.insert(t, TerrainFeature::Tree(valley_sim::terrain::Tree::new("1", 5)));
    select_tool(&mut w, ToolKind::Axe);
    w.farmer.facing = Dir::Right;
    for _ in 0..10 {
        swing(&mut w);
    }
    let Some(TerrainFeature::Tree(tree)) = w.location().terrain(t) else { panic!("tree gone") };
    assert!(tree.stump);
    for _ in 0..120 {
        w.update(&PlayerInput::default());
    }
    let on_ground: i32 = w.dropped.iter().filter(|d| d.item.object_id() == Some("388")).map(|d| d.item.stack()).sum();
    let held = w.farmer.inventory.count("388");
    assert!(held > 0, "some wood is collected automatically");
    assert!(held + on_ground >= 12, "wood: {held} + {on_ground}");
}

#[test]
fn passing_out_ends_day() {
    let (_, mut w) = new_world();
    w.warp_farmer("Farm", TilePos::new(20, 12), None);
    w.clock.time = 2550;
    for _ in 0..(60 * 8) {
        w.update(&PlayerInput::default());
    }
    assert_eq!(w.clock.day, 2);
    assert_eq!(w.current, "FarmHouse");
    let rep = w.last_report.clone().unwrap();
    assert!(rep.passed_out);
    assert_eq!(rep.money_lost, 50);
}

#[test]
fn npcs_follow_schedule_across_locations() {
    let (_, mut w) = new_world();
    let tobin = w.npcs.iter().position(|n| n.name == "Tobin").unwrap();
    assert_eq!(w.npcs[tobin].location, "SeedShop");
    // At 17:00 Tobin walks from the shop into town.
    w.clock.time = 1650;
    for _ in 0..(60 * 120) {
        w.update(&PlayerInput::default());
        if w.npcs[tobin].location == "Town" && !w.npcs[tobin].is_traveling() {
            break;
        }
    }
    assert_eq!(w.npcs[tobin].location, "Town");
    assert_eq!(w.npcs[tobin].tile(), TilePos::new(24, 9));
}

#[test]
fn talking_raises_friendship_once_per_day() {
    let (_, mut w) = new_world();
    let i = w.npcs.iter().position(|n| n.name == "Mara").unwrap();
    let home = w.npcs[i].home.tile;
    w.warp_farmer("Town", TilePos::new(home.x, home.y + 1), Some(Dir::Up));
    w.npcs[i].route.clear();
    w.npcs[i].set_tile(home);
    w.npcs[i].location = "Town".into();
    w.farmer.set_tile(TilePos::new(home.x, home.y + 1));
    w.farmer.facing = Dir::Up;
    w.drain_events();
    assert!(w.do_action(None));
    let ev = w.drain_events();
    let Some(WorldEvent::Dialogue { pages, .. }) = ev.iter().find(|e| matches!(e, WorldEvent::Dialogue { .. })) else { panic!("{ev:?}") };
    assert!(pages[0].text.contains("Welcome"));
    assert_eq!(w.farmer.friendships["Mara"].points, 20);
    w.do_action(None);
    assert_eq!(w.farmer.friendships["Mara"].points, 20);
}

#[test]
fn shop_stock_respects_season_and_buying() {
    let (_, mut w) = new_world();
    w.drain_events();
    w.open_shop("SeedShop");
    let ev = w.drain_events();
    let Some(WorldEvent::OpenShop(stock)) = ev.into_iter().find(|e| matches!(e, WorldEvent::OpenShop(_))) else { panic!() };
    let ids: Vec<_> = stock.entries.iter().filter_map(|e| e.item.object_id().map(String::from)).collect();
    assert!(ids.contains(&"Dev_TurnipSeeds".to_string()));
    assert!(!ids.contains(&"Dev_PumpkinSeeds".to_string()));
    let turnip = stock.entries.iter().find(|e| e.item.object_id() == Some("Dev_TurnipSeeds")).unwrap();
    assert_eq!(turnip.price, 20);
    assert!(w.buy(turnip, 10));
    assert_eq!(w.farmer.money, 300);
    assert_eq!(w.farmer.inventory.count("Dev_TurnipSeeds"), 10);
}

#[test]
fn save_and_restore_roundtrip() {
    let (mut cm, mut w) = new_world();
    w.warp_farmer("Farm", TilePos::new(12, 18), Some(Dir::Down));
    let t = TilePos::new(12, 19);
    clear_tile(&mut w, t);
    select_tool(&mut w, ToolKind::Hoe);
    swing(&mut w);
    w.farmer.money = 1234;
    w.end_day(false);
    let json = SaveData::capture(&w).to_json();
    let restored = SaveData::from_json(&json).unwrap().restore(&mut cm);
    assert_eq!(restored.farmer.money, 1234);
    assert_eq!(restored.clock.day, 2);
    let farm = &restored.locations["Farm"];
    assert_eq!(farm.state.objects.len(), w.locations["Farm"].state.objects.len());
    assert_eq!(SaveData::capture(&restored).farmer, SaveData::capture(&w).farmer);
}


#[test]
fn tv_shows_forecast() {
    let (_, mut w) = new_world();
    w.farmer.set_tile(TilePos::new(2, 3));
    w.farmer.facing = Dir::Up;
    w.drain_events();
    assert!(w.do_action(None));
    let ev = w.drain_events();
    assert!(ev.iter().any(|e| matches!(e, WorldEvent::TvMenu(_))), "{ev:?}");
    w.watch_channel(valley_sim::minecarts::Channel::Weather);
    let ev = w.drain_events();
    assert!(ev.iter().any(|e| matches!(e, WorldEvent::Message(m) if m.contains("Weather report"))), "{ev:?}");
}

#[test]
fn debris_uses_available_ids() {
    let (_, w) = new_world();
    let farm = &w.locations["Farm"];
    assert!(farm.state.objects.values().all(|o| w.items.objects.contains_key(&o.item_id)));
    assert!(!w.compat.bed_fallback, "placeholder farmhouse has a bed tile");
}

#[test]
fn npc_question_answers_change_friendship() {
    let (_, mut w) = new_world();
    w.clock.day = 3; // Wednesday
    let i = w.npcs.iter().position(|n| n.name == "Mara").unwrap();
    let home = w.npcs[i].home.tile;
    w.farmer.events_seen.push("Dev_Intro".into());
    w.warp_farmer("Town", TilePos::new(home.x, home.y + 1), Some(Dir::Up));
    w.npcs[i].route.clear();
    w.npcs[i].location = "Town".into();
    w.npcs[i].set_tile(home);
    w.farmer.set_tile(TilePos::new(home.x, home.y + 1));
    w.farmer.facing = Dir::Up;
    w.farmer.friendship("Mara").met = true;
    w.drain_events();
    assert!(w.do_action(None));
    let ev = w.drain_events();
    assert!(ev.iter().any(|e| matches!(e, WorldEvent::DialogueQuestion { choices, .. } if choices.len() == 2)), "{ev:?}");
    let before = w.farmer.friendships["Mara"].points;
    w.answer_question(0);
    assert_eq!(w.farmer.friendships["Mara"].points, before + 20);
    assert!(w.farmer.dialogue_answers.contains(&"7001".to_string()));
    let ev = w.drain_events();
    let Some(WorldEvent::Dialogue { pages, .. }) = ev.iter().find(|e| matches!(e, WorldEvent::Dialogue { .. })) else { panic!() };
    assert!(pages[0].text.contains("meteor"));
}
