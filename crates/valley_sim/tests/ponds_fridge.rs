use valley_content::ContentManager;
use valley_sim::item::Item;
use valley_sim::save::SaveData;
use valley_sim::world::{NewGame, World, WorldEvent, FRIDGE_TILE};
use valley_sim::{Dir, TilePos};

fn world() -> (ContentManager, World) {
    let mut cm = ContentManager::dev_only();
    let w = World::new(&mut cm, NewGame::default());
    (cm, w)
}

/// Build a finished pond at (12,18) and stand below it.
fn pond(w: &mut World) -> TilePos {
    w.farmer.money = 10_000;
    w.farmer.inventory.add(Item::object("390", 200));
    {
        let farm = w.locations.get_mut("Farm").unwrap();
        for y in 16..24 {
            for x in 10..18 {
                farm.state.objects.remove(&TilePos::new(x, y));
                farm.state.terrain.remove(&TilePos::new(x, y));
            }
        }
    }
    w.place_building("Dev_Pond", TilePos::new(12, 18)).unwrap();
    w.end_day(false);
    w.warp_farmer("Farm", TilePos::new(13, 21), Some(Dir::Up));
    TilePos::new(13, 20)
}

fn hold(w: &mut World, item: Item) {
    w.farmer.inventory.slots[9] = Some(item);
    w.farmer.current_slot = 9;
}

#[test]
fn fish_pond_grows_produces_and_asks_for_items() {
    let (_, mut w) = world();
    let t = pond(&mut w);
    assert!(w.is_pond("Dev_Pond"));
    // Empty pond: a non-fish item doesn't go in.
    hold(&mut w, Item::object("Dev_Berry", 3));
    assert!(w.do_action(Some(t)));
    assert!(w.buildings[0].pond.as_ref().map_or(true, |p| p.fish.is_none()));
    hold(&mut w, Item::object("Dev_Perch", 2));
    assert!(w.do_action(Some(t)));
    let p = w.buildings[0].pond.clone().unwrap();
    assert_eq!((p.fish.as_deref(), p.population), (Some("Dev_Perch"), 1));
    // A different fish is refused.
    hold(&mut w, Item::object("Dev_Darter", 1));
    w.drain_events();
    assert!(w.do_action(Some(t)));
    assert_eq!(w.buildings[0].pond.as_ref().unwrap().population, 1);
    // Perch spawn every 2 days up to the starting cap of 3, then ask for berries.
    for _ in 0..8 {
        w.end_day(false);
    }
    w.warp_farmer("Farm", TilePos::new(13, 21), Some(Dir::Up));
    let p = w.buildings[0].pond.clone().unwrap();
    assert_eq!(p.population, 3);
    let (want, n) = p.request.clone().expect("a population gate request");
    assert_eq!(want, "Dev_Berry");
    assert!((2..=4).contains(&n));
    // Collect produce first if there is any, then satisfy the request.
    if w.buildings[0].pond.as_ref().unwrap().output.is_some() {
        hold(&mut w, Item::object("Dev_Berry", 10));
        let before = w.farmer.inventory.slots.iter().flatten().count();
        assert!(w.do_action(Some(t)));
        assert!(w.farmer.inventory.slots.iter().flatten().count() >= before);
    }
    hold(&mut w, Item::object("Dev_Berry", 10));
    assert!(w.do_action(Some(t)));
    let p = w.buildings[0].pond.clone().unwrap();
    assert!(p.request.is_none());
    assert_eq!(p.cap, 5);
    assert_eq!(w.farmer.inventory.get(9).map(|i| i.stack()), Some(10 - n));
}

#[test]
fn pond_produce_appears_and_is_collected() {
    let (_, mut w) = world();
    let t = pond(&mut w);
    hold(&mut w, Item::object("Dev_Perch", 1));
    w.do_action(Some(t));
    let mut got = false;
    for _ in 0..10 {
        w.end_day(false);
        w.warp_farmer("Farm", TilePos::new(13, 21), Some(Dir::Up));
        if w.buildings[0].pond.as_ref().unwrap().output.is_some() {
            w.farmer.current_slot = 9;
            w.farmer.inventory.slots[9] = None;
            w.drain_events();
            assert!(w.do_action(Some(t)));
            assert!(w.buildings[0].pond.as_ref().unwrap().output.is_none());
            got = true;
            break;
        }
    }
    assert!(got, "a pond with fish produces something within ten days");
}

#[test]
fn pond_survives_save() {
    let (mut cm, mut w) = world();
    let t = pond(&mut w);
    hold(&mut w, Item::object("Dev_Perch", 1));
    w.do_action(Some(t));
    let json = SaveData::capture(&w).to_json();
    let r = SaveData::from_json(&json).unwrap().restore(&mut cm);
    assert_eq!(r.buildings[0].pond.as_ref().unwrap().fish.as_deref(), Some("Dev_Perch"));
}

#[test]
fn fridge_stores_items_and_supplies_cooking() {
    let (_, mut w) = world();
    // Upgrade the house to get the kitchen.
    w.farmer.money = 20_000;
    w.farmer.inventory.add(Item::object("388", 500));
    w.start_house_upgrade().unwrap();
    for _ in 0..3 {
        w.end_day(false);
    }
    assert_eq!(w.farmer.house_level, 1);
    // The kitchen fridge opens the shared chest menu.
    w.warp_farmer("FarmHouse", TilePos::new(14, 3), Some(Dir::Up));
    w.drain_events();
    assert!(w.do_action(Some(TilePos::new(14, 2))));
    assert!(w.drain_events().iter().any(|e| matches!(e, WorldEvent::OpenChest(t) if *t == FRIDGE_TILE)));
    // Move everything a recipe needs into the fridge, then cook.
    let (name, recipe) = w.cooking.iter().next().map(|(k, r)| (k.clone(), r.clone())).expect("a cooking recipe");
    w.farmer.cooking_recipes.insert(name.clone(), 0);
    for (ing, n) in &recipe.ingredients {
        let id = ing.trim_start_matches("(O)").to_string();
        let slot = 20;
        w.farmer.inventory.slots[slot] = Some(Item::object(&id, *n));
        w.chest_transfer(FRIDGE_TILE, true, slot);
        assert!(w.farmer.inventory.get(slot).is_none());
    }
    assert!(w.container(FRIDGE_TILE).unwrap().slots.iter().flatten().count() >= 1);
    assert!(!recipe.can_craft(&w.items, &w.farmer), "not from the backpack alone");
    assert!(w.cook(&name));
    assert!(w.farmer.fridge.slots.iter().flatten().count() == 0, "ingredients came from the fridge");
}
