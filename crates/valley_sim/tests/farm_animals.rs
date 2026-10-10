use valley_content::ContentManager;
use valley_sim::item::{Item, Tool, ToolKind};
use valley_sim::objects::ObjKind;
use valley_sim::save::SaveData;
use valley_sim::world::{NewGame, World};
use valley_sim::{Dir, TilePos};

fn world() -> (ContentManager, World) {
    let mut cm = ContentManager::dev_only();
    let w = World::new(&mut cm, NewGame::default());
    (cm, w)
}

fn clear_area(w: &mut World, x0: i32, y0: i32, wdt: i32, hgt: i32) {
    let farm = w.locations.get_mut("Farm").unwrap();
    for y in y0..y0 + hgt {
        for x in x0..x0 + wdt {
            farm.state.objects.remove(&TilePos::new(x, y));
            farm.state.terrain.remove(&TilePos::new(x, y));
        }
    }
}

fn build_coop(w: &mut World) -> String {
    w.farmer.money = 10_000;
    w.farmer.inventory.add(Item::object("388", 200));
    w.farmer.inventory.add(Item::object("390", 200));
    clear_area(w, 9, 17, 8, 6);
    assert!(w.place_building("Coop", TilePos::new(10, 18)).is_ok());
    assert_eq!(w.farmer.money, 9_000);
    assert_eq!(w.farmer.inventory.count("388"), 100);
    // Footprint blocks movement and other placement.
    assert!(w.place_building("Coop", TilePos::new(11, 18)).is_err());
    assert!(!valley_sim::collision::tile_passable(&w.locations["Farm"], TilePos::new(11, 19)));
    assert!(w.buildings[0].interior.is_none(), "takes a day to build");
    w.end_day(false);
    w.buildings[0].interior.clone().expect("interior exists after construction")
}

#[test]
fn build_enter_and_leave_a_coop() {
    let (_, mut w) = world();
    let interior = build_coop(&mut w);
    assert!(w.locations.contains_key(&interior));
    // Enter through the door (door at +2,+2 → tile 12,20); stand below it.
    w.warp_farmer("Farm", TilePos::new(12, 21), Some(Dir::Up));
    w.farmer.set_tile(TilePos::new(12, 21));
    w.farmer.facing = Dir::Up;
    assert!(w.do_action(None));
    assert_eq!(w.current, interior);
    // Walk out through the exit warp.
    for _ in 0..200 {
        w.update(&valley_sim::input::PlayerInput { move_y: 1, ..Default::default() });
        if w.current == "Farm" {
            break;
        }
    }
    assert_eq!(w.current, "Farm");
    assert_eq!(w.farmer.tile(), TilePos::new(12, 21));
}

#[test]
fn hen_lays_eggs_when_fed() {
    let (_, mut w) = world();
    let interior = build_coop(&mut w);
    assert!(w.buy_animal("Dev_Goat", None).is_err(), "no barn yet");
    assert!(w.buy_animal("Dev_Hen", Some("Pecky".into())).is_ok());
    assert_eq!(w.animals[0].name, "Pecky");
    assert_eq!(w.animals[0].home, interior);
    // Keep her inside (rainy) and feed via the trough each day.
    let mut eggs = 0;
    for _ in 0..5 {
        w.weather = valley_sim::time::Weather::Rain;
        let trough = TilePos::new(2, 2);
        w.locations.get_mut(&interior).unwrap().state.objects.insert(trough, valley_sim::objects::PlacedObject::new("Dev_Hay", false, ObjKind::Placed));
        w.end_day(false);
        eggs = w.locations[&interior].state.objects.values().filter(|o| o.item_id == "Dev_Egg").count();
    }
    assert!(eggs >= 2, "eggs: {eggs}");
    assert!(w.animals[0].age_days >= 5);
}

#[test]
fn petting_and_milking_goat() {
    let (_, mut w) = world();
    w.farmer.money = 50_000;
    w.farmer.inventory.add(Item::object("388", 500));
    w.farmer.inventory.add(Item::object("390", 500));
    clear_area(&mut w, 5, 16, 30, 12);
    assert!(w.place_building("Barn", TilePos::new(14, 18)).is_ok());
    w.end_day(false);
    w.end_day(false);
    let interior = w.buildings[0].interior.clone().unwrap();
    assert!(w.buy_animal("Dev_Goat", None).is_ok());
    // Make her an adult with milk ready.
    w.animals[0].age_days = 10;
    w.animals[0].produce = Some(Item::object("Dev_Milk", 1));
    w.warp_farmer(&interior, TilePos::new(6, 6), Some(Dir::Down));
    let t = TilePos::new(6, 7);
    w.animals[0].location = interior.clone();
    w.animals[0].pos = engine::Vec2::new(t.x as f32 * 64.0, t.y as f32 * 64.0 - 16.0);
    w.farmer.set_tile(TilePos::new(6, 6));
    w.farmer.facing = Dir::Down;
    assert!(w.do_action(None));
    assert!(w.animals[0].petted_today);
    assert_eq!(w.animals[0].friendship, 15);
    w.farmer.inventory.slots[11] = Some(Item::Tool(Tool::new(ToolKind::MilkPail)));
    w.farmer.current_slot = 11;
    w.apply_tool(ToolKind::MilkPail, 11, t);
    assert_eq!(w.farmer.inventory.count("Dev_Milk"), 1);
    assert!(w.animals[0].produce.is_none());
}

#[test]
fn buildings_and_animals_survive_save() {
    let (mut cm, mut w) = world();
    let interior = build_coop(&mut w);
    w.buy_animal("Dev_Hen", None).unwrap();
    let json = SaveData::capture(&w).to_json();
    let r = SaveData::from_json(&json).unwrap().restore(&mut cm);
    assert_eq!(r.buildings.len(), 1);
    assert!(r.locations.contains_key(&interior));
    assert_eq!(r.animals.len(), 1);
    assert!(!valley_sim::collision::tile_passable(&r.locations["Farm"], TilePos::new(11, 19)));
}


