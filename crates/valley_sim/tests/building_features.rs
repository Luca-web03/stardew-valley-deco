use valley_content::ContentManager;
use valley_sim::item::Item;
use valley_sim::objects::ObjKind;
use valley_sim::world::{NewGame, World};
use valley_sim::{Dir, TilePos};

fn world() -> World {
    let mut cm = ContentManager::dev_only();
    let mut w = World::new(&mut cm, NewGame::default());
    w.farmer.money = 100_000;
    w.farmer.inventory.add(Item::object("388", 900));
    w.farmer.inventory.add(Item::object("390", 900));
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

#[test]
fn coops_upgrade_in_place_keeping_their_hens() {
    let mut w = world();
    assert!(w.upgrade_building("Dev_BigCoop", None).is_err(), "nothing to upgrade yet");
    w.place_building("Coop", TilePos::new(10, 18)).unwrap();
    w.end_day(false);
    w.buy_animal("Dev_Hen", Some("Pecky".into())).unwrap();
    let interior = w.buildings[0].interior.clone().unwrap();
    w.locations.get_mut(&interior).unwrap().state.objects.insert(TilePos::new(3, 3), valley_sim::objects::PlacedObject::forage("Dev_Egg"));
    let id = w.upgrade_building("Dev_BigCoop", None).unwrap();
    assert_eq!(w.buildings[0].upgrade_to.as_deref(), Some("Dev_BigCoop"));
    w.end_day(false);
    w.end_day(false);
    let b = w.buildings.iter().find(|b| b.id == id).unwrap();
    assert_eq!(b.kind, "Dev_BigCoop");
    assert_eq!(b.interior.as_deref(), Some(interior.as_str()), "same interior");
    let loc = &w.locations[&interior];
    assert_eq!(loc.width(), 14, "the bigger map");
    assert!(w.animals.iter().any(|a| a.name == "Pecky" && a.home == interior));
    // Room for more hens now.
    for _ in 0..6 {
        w.buy_animal("Dev_Hen", None).unwrap();
    }
}

#[test]
fn mills_grind_overnight_through_their_chests() {
    let mut w = world();
    w.place_building("Dev_Mill", TilePos::new(10, 18)).unwrap();
    w.end_day(false);
    // The collision map leaves one tile open.
    let farm = &w.locations["Farm"];
    assert!(valley_sim::collision::tile_passable(farm, TilePos::new(11, 19)));
    assert!(!valley_sim::collision::tile_passable(farm, TilePos::new(10, 19)));
    w.warp_farmer("Farm", TilePos::new(10, 20), Some(Dir::Up));
    w.farmer.inventory.slots[8] = Some(Item::object("Dev_Bean", 2));
    w.farmer.current_slot = 8;
    assert!(w.do_action(Some(TilePos::new(10, 19))));
    assert_eq!(w.farmer.inventory.count("Dev_Bean"), 2, "only turnips");
    w.farmer.inventory.slots[8] = Some(Item::object("Dev_Turnip", 3));
    assert!(w.do_action(Some(TilePos::new(10, 19))));
    assert_eq!(w.farmer.inventory.count("Dev_Turnip"), 0);
    w.end_day(false);
    let id = w.buildings[0].id;
    assert_eq!(w.building_chest(id, "Output").map(|i| i.count("Dev_Bread")), Some(3));
    w.warp_farmer("Farm", TilePos::new(13, 20), Some(Dir::Up));
    assert!(w.do_action(Some(TilePos::new(13, 19))));
    assert!(w.farmer.inventory.count("Dev_Bread") >= 3);
}

#[test]
fn silos_store_hay_from_data_and_indoor_items_appear() {
    let mut w = world();
    w.farmer.inventory.add(Item::object("Dev_CopperBar", 20));
    assert_eq!(w.hay_capacity(), 0);
    w.place_building("Silo", TilePos::new(20, 18)).unwrap();
    assert_eq!(w.hay_capacity(), 0, "not while it's being built");
    w.end_day(false);
    assert_eq!(w.hay_capacity(), 240);
    // A big coop built fresh gets its incubator.
    w.place_building("Dev_BigCoop", TilePos::new(10, 18)).unwrap();
    w.end_day(false);
    w.end_day(false);
    w.end_day(false);
    let b = w.buildings.iter().find(|b| b.kind == "Dev_BigCoop").unwrap();
    assert_eq!(b.days_left, 0);
    let interior = b.interior.clone().unwrap();
    let inc = w.locations[&interior].state.objects.get(&TilePos::new(11, 2)).expect("incubator placed");
    assert_eq!(inc.item_id, "Dev_Incubator");
    assert!(matches!(inc.kind, ObjKind::Machine { .. } | ObjKind::Placed));
}

#[test]
fn skins_change_a_buildings_look_and_can_be_built_directly() {
    let mut w = world();
    let skins = w.building_skins("Coop");
    assert_eq!(skins, vec![("Dev_BlueCoop".to_string(), "Blue Henhouse".to_string())], "conditions filter skins");
    let d = w.construction_entry("Coop|Dev_BlueCoop").unwrap();
    assert_eq!((d.name.as_str(), d.build_cost), ("Blue Henhouse", 1500));
    let money = w.farmer.money;
    w.place_building("Coop|Dev_BlueCoop", TilePos::new(10, 18)).unwrap();
    assert_eq!(w.farmer.money, money - 1500);
    let b = w.buildings[0].clone();
    assert_eq!((b.kind.as_str(), b.skin.as_deref()), ("Coop", Some("Dev_BlueCoop")));
    assert_eq!(w.building_texture(&b).as_deref(), Some("Buildings\\Dev_Coop_Blue"));
    // Back to the default look; unavailable looks are refused.
    w.set_building_skin(b.id, None).unwrap();
    assert_eq!(w.building_texture(&w.buildings[0]).as_deref(), Some("Buildings\\Dev_Coop"));
    assert!(w.set_building_skin(b.id, Some("Dev_StoneCoop")).is_err());
}
