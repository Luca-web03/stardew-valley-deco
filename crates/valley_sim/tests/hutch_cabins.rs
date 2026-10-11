use valley_content::ContentManager;
use valley_sim::item::{Item, Tool, ToolKind};
use valley_sim::objects::{ObjKind, PlacedObject};
use valley_sim::world::{NewGame, World};
use valley_sim::{Dir, TilePos};

fn world() -> World {
    let mut cm = ContentManager::dev_only();
    let mut w = World::new(&mut cm, NewGame::default());
    w.farmer.money = 100_000;
    w.farmer.inventory.add(Item::object("390", 400));
    w.farmer.inventory.add(Item::object("388", 100));
    w.farmer.inventory.add(Item::object("Dev_IronBar", 5));
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

fn build(w: &mut World, kind: &str, at: TilePos) -> String {
    w.place_building(kind, at).unwrap();
    let i = w.buildings.len() - 1;
    while w.buildings[i].interior.is_none() {
        w.end_day(false);
    }
    w.buildings[i].interior.clone().unwrap()
}

#[test]
fn incubators_hatch_slimes_and_troughs_make_slime_balls() {
    let mut w = world();
    let hutch = build(&mut w, "Dev_SlimeHutch", TilePos::new(10, 18));
    w.warp_farmer(&hutch, TilePos::new(4, 4), Some(Dir::Down));
    // Incubator in front of the farmer, then the egg.
    let inc = TilePos::new(4, 5);
    w.location_mut().state.objects.insert(inc, PlacedObject::new("Dev_Incubator", true, ObjKind::Placed));
    w.farmer.inventory.slots[5] = Some(Item::object("Dev_SlimeEgg", 1));
    w.farmer.current_slot = 5;
    assert!(w.do_action(Some(inc)));
    assert_eq!(w.farmer.inventory.count("Dev_SlimeEgg"), 0);
    w.update_ms(&Default::default(), 16); // brings out the (zero) slimes
    w.advance_machines(valley_sim::slime_hutch::INCUBATION_MINUTES);
    let bi = w.hutch_of(&hutch).unwrap();
    assert_eq!(w.buildings[bi].slimes, vec!["Dev_Blob".to_string()]);
    assert_eq!(w.monsters.iter().filter(|m| m.location == hutch).count(), 1, "hatched slime is out");

    // Fill every trough with the watering can.
    let troughs = w.troughs(&hutch);
    assert_eq!(troughs.len(), 4);
    let mut can = Tool::new(ToolKind::WateringCan);
    can.water = 40;
    w.farmer.inventory.slots[0] = Some(Item::Tool(can));
    for t in &troughs {
        w.apply_tool(ToolKind::WateringCan, 0, *t);
    }
    assert_eq!(w.buildings[bi].watered_troughs.len(), 4);
    // More slimes make balls certain.
    w.buildings[bi].slimes.extend(["Dev_Blob".to_string(), "Dev_Blob".to_string(), "Dev_Blob".to_string()]);
    w.spawned_hutches.clear();
    w.monsters.retain(|m| m.location != hutch);
    w.end_day(false);
    let bi = w.hutch_of(&hutch).unwrap();
    assert!(w.buildings[bi].watered_troughs.is_empty(), "troughs dry overnight");
    let balls: Vec<TilePos> = w.locations[&hutch].state.objects.iter().filter(|(_, o)| o.item_id == "Dev_SlimeBall").map(|(p, _)| *p).collect();
    assert_eq!(balls.len(), 4);
    // Slimes come back out on entering, and survive the night.
    w.warp_farmer(&hutch, TilePos::new(4, 4), Some(Dir::Down));
    w.update_ms(&Default::default(), 16);
    assert_eq!(w.monsters.iter().filter(|m| m.location == hutch).count(), 4);
    // Squish a ball.
    let ball = balls[0];
    w.warp_farmer(&hutch, TilePos::new(ball.x, ball.y - 1), Some(Dir::Down));
    w.dropped.clear();
    assert!(w.do_action(Some(ball)));
    let goo: i32 = w.dropped.iter().filter(|d| d.item.object_id() == Some("Dev_Goo")).map(|d| d.item.stack()).sum();
    assert!((10..=20).contains(&goo), "{goo} goo");
}

#[test]
fn incubators_only_work_in_a_hutch() {
    let mut w = world();
    w.warp_farmer("Farm", TilePos::new(20, 20), Some(Dir::Down));
    let t = TilePos::new(20, 21);
    w.location_mut().state.objects.insert(t, PlacedObject::new("Dev_Incubator", true, ObjKind::Placed));
    w.farmer.inventory.slots[5] = Some(Item::object("Dev_SlimeEgg", 1));
    w.farmer.current_slot = 5;
    assert!(w.do_action(Some(t)));
    assert_eq!(w.farmer.inventory.count("Dev_SlimeEgg"), 1);
}

#[test]
fn farmhands_live_in_cabins() {
    let mut w = world();
    let cabin = build(&mut w, "Dev_Cabin", TilePos::new(10, 18));
    let a = w.add_farmhand("Pal");
    assert_eq!(w.farmhands[a].current, cabin);
    assert_eq!(w.farmhands[a].farmer.home_location.as_deref(), Some(cabin.as_str()));
    assert_eq!(w.buildings.last().unwrap().owner.as_deref(), Some("Pal"));
    // A second farmhand has no cabin and sleeps in the farmhouse...
    let b = w.add_farmhand("Sam");
    assert_eq!(w.farmhands[b].current, "FarmHouse");
    w.as_farmhand(a, |w| w.warp_farmer("Town", TilePos::new(10, 10), None));
    w.end_day(false);
    assert_eq!(w.farmhands[a].current, cabin, "wakes up at home");
    // ...until a second cabin goes up.
    let cabin2 = build(&mut w, "Dev_Cabin", TilePos::new(16, 18));
    w.end_day(false);
    assert_eq!(w.farmhands[b].current, cabin2);
    // Rejoining by name finds the same farmhand.
    assert_eq!(w.farmhand_index("Sam"), Some(b));
    // The cabin bed is a bed.
    let farmer = w.farmhands[a].farmer.tile();
    let loc = &w.locations[&cabin];
    assert!(valley_sim::collision::tile_passable(loc, farmer));
}

#[test]
fn builders_offer_their_own_buildings() {
    let mut w = world();
    w.drain_events();
    assert!(w.run_action_pub("Carpenter"));
    let robin = w.drain_events().into_iter().find_map(|e| match e {
        valley_sim::world::WorldEvent::OpenCarpenter(l) => Some(l),
        _ => None,
    }).unwrap();
    assert!(robin.contains(&"Coop".to_string()));
    assert!(!robin.contains(&"Dev_Obelisk".to_string()), "the wizard builds obelisks");
    assert!(w.run_action_pub("WizardBook"));
    let wizard = w.drain_events().into_iter().find_map(|e| match e {
        valley_sim::world::WorldEvent::OpenCarpenter(l) => Some(l),
        _ => None,
    }).unwrap();
    assert!(wizard.contains(&"Dev_Obelisk".to_string()) && wizard.contains(&"Dev_HarvestHut".to_string()));
    assert!(!wizard.contains(&"Coop".to_string()));
}
