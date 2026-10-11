use valley_content::ContentManager;
use valley_sim::item::Item;
use valley_sim::objects::ObjKind;
use valley_sim::world::{NewGame, World, WorldEvent};
use valley_sim::{Dir, TilePos};

fn world() -> World {
    let mut cm = ContentManager::dev_only();
    let mut w = World::new(&mut cm, NewGame::default());
    let farm = w.locations.get_mut("Farm").unwrap();
    for y in 14..32 {
        for x in 6..36 {
            farm.state.objects.remove(&TilePos::new(x, y));
            farm.state.terrain.remove(&TilePos::new(x, y));
        }
    }
    farm.state.clumps.clear();
    w
}

fn place(w: &mut World, big: &str, t: TilePos) {
    w.warp_farmer("Farm", TilePos::new(t.x, t.y - 1), Some(Dir::Down));
    w.farmer.inventory.slots[6] = Some(Item::big(big, 1));
    w.farmer.current_slot = 6;
    assert!(w.do_action(Some(t)), "placed {big}");
}

fn output(w: &World, loc: &str, t: TilePos) -> Option<(Item, bool)> {
    match &w.locations[loc].state.objects.get(&t)?.kind {
        ObjKind::Machine { output: Some(o), ready, .. } => Some((o.clone(), *ready)),
        _ => None,
    }
}

#[test]
fn seed_maker_turns_crops_into_their_seeds() {
    let mut w = world();
    let t = TilePos::new(20, 20);
    place(&mut w, "Dev_SeedMaker", t);
    w.farmer.inventory.slots[7] = Some(Item::object("Dev_Turnip", 3));
    w.farmer.current_slot = 7;
    assert!(w.do_action(Some(t)));
    assert_eq!(w.farmer.inventory.count("Dev_Turnip"), 2);
    w.advance_machines(30);
    let (out, ready) = output(&w, "Farm", t).expect("seeds");
    assert!(ready);
    let id = out.object_id().unwrap();
    assert!(id == "Dev_TurnipSeeds" || id == "770" || id == "499", "{id}");
}

#[test]
fn mushroom_boxes_produce_every_day() {
    let mut w = world();
    let t = TilePos::new(20, 20);
    place(&mut w, "Dev_MushroomBox", t);
    let (out, ready) = output(&w, "Farm", t).expect("starts on put-down");
    assert_eq!(out.object_id(), Some("Dev_Mushroom"));
    assert!(!ready);
    w.end_day(false);
    assert!(output(&w, "Farm", t).unwrap().1, "ready next morning");
    w.warp_farmer("Farm", TilePos::new(t.x, t.y - 1), Some(Dir::Down));
    w.farmer.current_slot = 9;
    assert!(w.do_action(Some(t)));
    assert_eq!(w.farmer.inventory.count("Dev_Mushroom"), 1);
    assert!(output(&w, "Farm", t).is_none(), "idle until tomorrow");
    w.end_day(false);
    assert!(output(&w, "Farm", t).is_some(), "day update restarts it");
}

#[test]
fn the_saloon_style_dish_of_the_day_is_in_stock() {
    let mut w = world();
    w.open_shop("SeedShop");
    let stock = w.drain_events().into_iter().find_map(|e| match e {
        WorldEvent::OpenShop(s) => Some(s),
        _ => None,
    });
    let stock = stock.expect("shop opens");
    let cooked: Vec<_> = stock.entries.iter().filter(|e| e.item.object_id().and_then(|id| w.items.object(id)).is_some_and(|o| o.data.category == -7) && e.limit == 3).collect();
    assert_eq!(cooked.len(), 1, "one dish of the day");
}

#[test]
fn qi_board_offers_gem_orders() {
    let mut w = world();
    assert!(w.run_action_pub("SpecialOrders Qi"));
    assert_eq!(w.board_offers(), &["Dev_QiTrial".to_string()]);
    assert!(w.run_action_pub("SpecialOrders"));
    assert!(!w.board_offers().contains(&"Dev_QiTrial".to_string()), "not on the town board");
    w.accept_order("Dev_QiTrial").unwrap();
    let gems = w.farmer.qi_gems;
    for _ in 0..25 {
        w.order_slay("Dev_Blob");
    }
    assert_eq!(w.farmer.qi_gems, gems + 5);
}

#[test]
fn the_farm_cave_offers_mushrooms_or_bats() {
    for choice in ["mushrooms", "bats"] {
        let mut w = world();
        w.warp_farmer("FarmCave", TilePos::new(6, 8), Some(Dir::Up));
        w.update_ms(&Default::default(), 16);
        assert!(!w.drain_events().iter().any(|e| matches!(e, WorldEvent::Choice { .. })), "not before 25,000g");
        w.farmer.total_earned = 30_000;
        w.update_ms(&Default::default(), 16);
        let offered = w.drain_events().into_iter().any(|e| matches!(e, WorldEvent::Choice { options, .. } if options.len() == 2));
        assert!(offered);
        w.choose(&format!("cave:{choice}"));
        assert_eq!(w.farmer.cave_choice.as_deref(), Some(choice));
        let boxes = w.locations["FarmCave"].state.objects.values().filter(|o| o.item_id == "Dev_MushroomBox").count();
        if choice == "mushrooms" {
            assert_eq!(boxes, valley_sim::farm_cave::MUSHROOM_BOXES);
            w.end_day(false);
            let ready = w.locations["FarmCave"].state.objects.values().filter(|o| matches!(o.kind, ObjKind::Machine { ready: true, .. })).count();
            assert!(ready > 0, "boxes grow mushrooms");
        } else {
            assert_eq!(boxes, 0);
            for _ in 0..6 {
                w.end_day(false);
            }
            let fruit = w.locations["FarmCave"].state.objects.values().filter(|o| matches!(o.kind, ObjKind::Forage)).count();
            assert!(fruit > 0, "bats bring fruit");
        }
    }
}

#[test]
fn bee_houses_make_honey_flavored_by_nearby_flowers() {
    use valley_sim::crop::Crop;
    use valley_sim::terrain::{HoeDirt, TerrainFeature};
    let mut w = world();
    let t = TilePos::new(20, 20);
    place(&mut w, "Dev_BeeHouse", t);
    for _ in 0..4 {
        w.end_day(false);
    }
    let (out, ready) = output(&w, "Farm", t).expect("honey");
    assert!(ready);
    w.warp_farmer("Farm", TilePos::new(20, 19), Some(Dir::Down));
    assert!(w.do_action(Some(t)));
    let wild = w.farmer.inventory.slots.iter().flatten().find(|i| i.object_id() == Some("340")).cloned().expect("collected");
    assert_eq!(w.items.display_name(&wild), "Wild Honey");
    assert_eq!(out.object_id(), Some("340"));
    assert!(output(&w, "Farm", t).is_some(), "restarts on collection");

    // A grown flower three tiles away flavors the next batch.
    let data = w.items.crops["Dev_SunbloomSeeds"].clone();
    let mut rng = engine::Rng::new(1);
    let mut crop = Crop::new("Dev_SunbloomSeeds", &mut rng);
    crop.grow_fully(&data);
    let mut dirt = HoeDirt::new();
    dirt.crop = Some(crop);
    w.locations.get_mut("Farm").unwrap().state.terrain.insert(TilePos::new(22, 21), TerrainFeature::HoeDirt(dirt));
    for _ in 0..4 {
        w.end_day(false);
    }
    w.warp_farmer("Farm", TilePos::new(20, 19), Some(Dir::Down));
    w.farmer.inventory.slots.iter_mut().for_each(|s| {
        if s.as_ref().and_then(|i| i.object_id()) == Some("340") {
            *s = None;
        }
    });
    assert!(w.do_action(Some(t)));
    let honey = w.farmer.inventory.slots.iter().flatten().find(|i| i.object_id() == Some("340")).cloned().unwrap();
    assert_eq!(w.items.display_name(&honey), "Sunbloom Honey");
    assert_eq!(w.items.sell_price(&honey), 100 + 60 * 2);
}
