use valley_content::ContentManager;
use valley_sim::input::PlayerInput;
use valley_sim::item::Item;
use valley_sim::quests::Objective;
use valley_sim::terrain::{HoeDirt, TerrainFeature};
use valley_sim::world::{NewGame, World, WorldEvent};
use valley_sim::{Dir, TilePos};

fn world() -> World {
    let mut cm = ContentManager::dev_only();
    World::new(&mut cm, NewGame { farmer_name: "Kai".into(), ..Default::default() })
}

#[test]
fn cooking_consumes_ingredients_and_food_buffs_expire() {
    let mut w = world();
    assert!(w.farmer.cooking_recipes.contains_key("Garden Salad"));
    assert!(!w.cook("Garden Salad"));
    w.farmer.inventory.add(Item::object("Dev_Turnip", 1));
    w.farmer.inventory.add(Item::object("Dev_Bean", 1));
    assert!(w.cook("Garden Salad"));
    assert_eq!(w.farmer.inventory.count("Dev_Turnip"), 0);
    let slot = (0..36).find(|&i| w.farmer.inventory.get(i).and_then(|x| x.object_id()) == Some("Dev_Salad")).unwrap();
    w.eat(slot);
    assert_eq!(w.farmer.buff_totals().speed, 1);
    // 240 minutes = 24 ten-minute ticks.
    for _ in 0..25 {
        w.update_ms(&PlayerInput::default(), valley_sim::time::TEN_MINUTES_MS);
    }
    assert!(w.farmer.buffs.is_empty());
}

#[test]
fn kitchen_action_opens_cooking() {
    let mut w = world();
    // The kitchen comes with the first farmhouse upgrade.
    w.farmer.money = 20_000;
    w.farmer.inventory.add(valley_sim::item::Item::object("388", 450));
    w.start_house_upgrade().unwrap();
    for _ in 0..3 {
        w.end_day(false);
    }
    assert_eq!(w.farmer.house_level, 1);
    w.farmer.set_tile(TilePos::new(12, 3));
    w.farmer.facing = Dir::Up;
    w.drain_events();
    assert!(w.do_action(None));
    assert!(w.drain_events().contains(&WorldEvent::OpenCooking));
}

#[test]
fn welcome_letter_then_dated_letter_with_items() {
    let mut w = world();
    w.warp_farmer("Farm", TilePos::new(25, 10), Some(Dir::Up));
    w.farmer.set_tile(TilePos::new(25, 10));
    w.farmer.facing = Dir::Up;
    w.drain_events();
    assert!(w.do_action(None));
    let ev = w.drain_events();
    let Some(WorldEvent::Letter { letter, .. }) = ev.iter().find(|e| matches!(e, WorldEvent::Letter { .. })) else { panic!("{ev:?}") };
    assert!(letter.text.contains("Kai"));
    w.end_day(false);
    assert!(w.farmer.mailbox.contains(&"spring_2".to_string()));
    w.warp_farmer("Farm", TilePos::new(25, 10), Some(Dir::Up));
    w.farmer.set_tile(TilePos::new(25, 10));
    w.farmer.facing = Dir::Up;
    w.drain_events();
    w.do_action(None);
    assert_eq!(w.farmer.inventory.count("Dev_TurnipSeeds"), 10);
}

#[test]
fn harvest_quest_chains_into_delivery() {
    let mut w = world();
    assert!(matches!(w.farmer.quests[0].objective, Objective::Harvest { .. }));
    // Grow and harvest a turnip.
    w.warp_farmer("Farm", TilePos::new(12, 18), Some(Dir::Down));
    w.farmer.set_tile(TilePos::new(12, 18));
    w.farmer.facing = Dir::Down;
    let t = TilePos::new(12, 19);
    w.location_mut().state.objects.remove(&t);
    let mut dirt = HoeDirt::new();
    let mut rng = engine::Rng::new(1);
    let mut crop = valley_sim::crop::Crop::new("Dev_TurnipSeeds", &mut rng);
    crop.phase = 4;
    dirt.crop = Some(crop);
    w.location_mut().state.terrain.insert(t, TerrainFeature::HoeDirt(dirt));
    w.farmer.current_slot = 11;
    let money = w.farmer.money;
    assert!(w.do_action(None));
    assert_eq!(w.farmer.money, money + 50);
    assert_eq!(w.farmer.quests.len(), 1);
    assert!(matches!(w.farmer.quests[0].objective, Objective::Deliver { .. }));
    // Deliver to Mara.
    let i = w.npcs.iter().position(|n| n.name == "Mara").unwrap();
    let home = w.npcs[i].home.tile;
    w.warp_farmer("Town", TilePos::new(home.x, home.y + 1), Some(Dir::Up));
    w.npcs[i].route.clear();
    w.npcs[i].location = "Town".into();
    w.npcs[i].set_tile(home);
    w.farmer.set_tile(TilePos::new(home.x, home.y + 1));
    w.farmer.facing = Dir::Up;
    let slot = (0..36).find(|&k| w.farmer.inventory.get(k).and_then(|x| x.object_id()) == Some("Dev_Turnip")).unwrap();
    w.farmer.current_slot = slot;
    w.drain_events();
    assert!(w.do_action(None));
    assert!(w.farmer.quests.is_empty());
    assert_eq!(w.farmer.money, money + 150);
    assert!(w.drain_events().iter().any(|e| matches!(e, WorldEvent::Dialogue { .. })));
}

#[test]
fn help_wanted_quest_expires() {
    let mut w = world();
    for _ in 0..10 {
        if w.daily_quest.is_some() {
            break;
        }
        w.end_day(false);
    }
    assert!(w.accept_daily_quest());
    let n = w.farmer.quests.len();
    w.end_day(false);
    w.end_day(false);
    assert_eq!(w.farmer.quests.len(), n - 1);
}
