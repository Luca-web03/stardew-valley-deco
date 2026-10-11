use valley_content::ContentManager;
use valley_sim::item::Item;
use valley_sim::objects::{ObjKind, PlacedObject, SprinklerShape};
use valley_sim::terrain::{HoeDirt, TerrainFeature};
use valley_sim::world::{NewGame, World};
use valley_sim::{Dir, TilePos};

fn world() -> World {
    let mut cm = ContentManager::dev_only();
    let mut w = World::new(&mut cm, NewGame::default());
    w.farmer.money = 100_000;
    w.farmer.inventory.add(Item::object("388", 400));
    w.farmer.inventory.add(Item::object("390", 400));
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

fn barn(w: &mut World) -> String {
    w.place_building("Barn", TilePos::new(14, 18)).unwrap();
    w.end_day(false);
    w.end_day(false);
    w.buildings[0].interior.clone().unwrap()
}

#[test]
fn auto_grabbers_gather_and_auto_petters_pet() {
    let mut w = world();
    let home = barn(&mut w);
    w.buy_animal("Dev_Goat", None).unwrap();
    w.animals[0].age_days = 10;
    w.animals[0].produce = Some(Item::object("Dev_Milk", 1));
    let loc = w.locations.get_mut(&home).unwrap();
    let grab = TilePos::new(3, 3);
    loc.state.objects.insert(grab, PlacedObject::new("Dev_AutoGrabber", true, ObjKind::Chest(valley_sim::inventory::Inventory::new(36))));
    loc.state.objects.insert(TilePos::new(5, 3), PlacedObject::new("Dev_AutoPetter", true, ObjKind::Placed));
    loc.state.objects.insert(TilePos::new(6, 6), PlacedObject::forage("Dev_Egg"));
    w.start_day();
    let loc = &w.locations[&home];
    let ObjKind::Chest(inv) = &loc.object(grab).unwrap().kind else { panic!() };
    assert_eq!(inv.count("Dev_Milk"), 1, "milk taken off the goat");
    assert_eq!(inv.count("Dev_Egg"), 1, "floor produce gathered");
    assert!(loc.object(TilePos::new(6, 6)).is_none());
    assert!(w.animals[0].produce.is_none());
    assert!(w.animals[0].petted_today);
}

#[test]
fn placing_an_auto_grabber_makes_a_chest() {
    let mut w = world();
    w.warp_farmer("Farm", TilePos::new(20, 20), Some(Dir::Down));
    w.farmer.inventory.slots[8] = Some(Item::big("Dev_AutoGrabber", 1));
    w.farmer.current_slot = 8;
    assert!(w.place_current(TilePos::new(20, 21)));
    assert!(matches!(w.location().object(TilePos::new(20, 21)).unwrap().kind, ObjKind::Chest(_)));
}

#[test]
fn mini_bins_ship_and_workbenches_craft_from_nearby_chests() {
    let mut w = world();
    w.warp_farmer("Farm", TilePos::new(20, 20), Some(Dir::Down));
    let bin = TilePos::new(20, 21);
    w.location_mut().state.objects.insert(bin, PlacedObject::new("Dev_MiniBin", true, ObjKind::Placed));
    w.farmer.inventory.slots[8] = Some(Item::object("Dev_Turnip", 3));
    w.farmer.current_slot = 8;
    assert!(w.do_action(Some(bin)));
    assert_eq!(w.shipping_bin.len(), 1);
    assert_eq!(w.farmer.inventory.count("Dev_Turnip"), 0);

    // A workbench with wood in the chest beside it.
    let bench = TilePos::new(22, 21);
    w.location_mut().state.objects.insert(bench, PlacedObject::new("Dev_Workbench", true, ObjKind::Placed));
    let mut chest = valley_sim::inventory::Inventory::new(36);
    chest.add(Item::object("388", 50));
    w.location_mut().state.objects.insert(TilePos::new(23, 21), PlacedObject::new("Dev_Box", true, ObjKind::Chest(chest)));
    let wood = w.farmer.inventory.count("388");
    w.farmer.inventory.remove_object("388", wood);
    w.farmer.crafting_recipes.insert("Storage Box".into(), 0);
    assert!(!w.can_craft_here("Storage Box"));
    w.warp_farmer("Farm", TilePos::new(22, 20), Some(Dir::Down));
    w.drain_events();
    assert!(w.do_action(Some(bench)));
    assert!(w.drain_events().iter().any(|e| matches!(e, valley_sim::world::WorldEvent::OpenCrafting)));
    assert!(w.can_craft_here("Storage Box"));
    assert!(w.craft("Storage Box"));
    let ObjKind::Chest(inv) = &w.location().object(TilePos::new(23, 21)).unwrap().kind else { panic!() };
    assert_eq!(inv.count("388"), 30, "wood came from the chest");
}

#[test]
fn nozzles_widen_and_enrichers_fertilize() {
    let mut w = world();
    w.warp_farmer("Farm", TilePos::new(20, 19), Some(Dir::Down));
    let s = TilePos::new(20, 20);
    w.location_mut().state.objects.insert(s, PlacedObject::new("Dev_Sprinkler", false, ObjKind::Sprinkler { radius: SprinklerShape::Cross, attachment: None, fertilizer: None }));
    for y in 18..23 {
        for x in 18..23 {
            if (x, y) != (20, 20) {
                w.location_mut().state.terrain.insert(TilePos::new(x, y), TerrainFeature::HoeDirt(HoeDirt::new()));
            }
        }
    }
    w.farmer.inventory.slots[8] = Some(Item::object("Dev_Nozzle", 1));
    w.farmer.current_slot = 8;
    assert!(w.do_action(Some(s)));
    w.run_sprinklers();
    let watered = |w: &World, x, y| matches!(w.location().terrain(TilePos::new(x, y)), Some(TerrainFeature::HoeDirt(d)) if d.watered);
    assert!(watered(&w, 21, 21), "diagonals with a nozzle");
    assert!(!watered(&w, 22, 22));
    // Swap in an enricher and give it fertilizer.
    w.farmer.inventory.slots[8] = Some(Item::object("Dev_Enricher", 1));
    assert!(w.do_action(Some(s)));
    assert_eq!(w.farmer.inventory.count("Dev_Nozzle"), 1, "the nozzle comes back");
    w.farmer.inventory.slots[8] = Some(Item::object("Dev_Fertilizer", 10));
    assert!(w.do_action(Some(s)));
    w.run_sprinklers();
    let fert = |w: &World, x, y| matches!(w.location().terrain(TilePos::new(x, y)), Some(TerrainFeature::HoeDirt(d)) if d.fertilizer.is_some());
    assert!(fert(&w, 20, 21) && fert(&w, 19, 20));
    assert!(!fert(&w, 21, 21), "plain area without the nozzle");
    let left = match &w.location().object(s).unwrap().kind {
        ObjKind::Sprinkler { fertilizer, .. } => fertilizer.as_ref().map(|f| f.stack()).unwrap_or(0),
        _ => 0,
    };
    assert_eq!(left, 6);
}
