use valley_content::ContentManager;
use valley_sim::item::Item;
use valley_sim::objects::{ObjKind, PlacedObject};
use valley_sim::world::{NewGame, World};
use valley_sim::{Dir, TilePos};

fn world() -> World {
    let mut cm = ContentManager::dev_only();
    let mut w = World::new(&mut cm, NewGame::default());
    w.event = None;
    w.farmer.money = 100_000;
    w.farmer.inventory.add(Item::object("388", 900));
    w.farmer.inventory.add(Item::object("390", 900));
    let farm = w.locations.get_mut("Farm").unwrap();
    for y in 10..32 {
        for x in 6..40 {
            farm.state.objects.remove(&TilePos::new(x, y));
            farm.state.terrain.remove(&TilePos::new(x, y));
        }
    }
    farm.state.clumps.clear();
    w
}

fn hold(w: &mut World, item: Item) {
    w.farmer.inventory.slots[11] = Some(item);
    w.farmer.current_slot = 11;
}

#[test]
fn a_stardrop_raises_maximum_energy() {
    let mut w = world();
    let max = w.farmer.max_stamina;
    w.farmer.stamina = 10.0;
    hold(&mut w, Item::object("Dev_Stardrop", 1));
    w.eat(11);
    assert_eq!(w.farmer.max_stamina, max + 34);
    assert_eq!(w.farmer.stamina, (max + 34) as f32);
}

#[test]
fn golden_crackers_double_a_hens_eggs() {
    let mut w = world();
    w.place_building("Coop", TilePos::new(10, 18)).unwrap();
    w.end_day(false);
    w.buy_animal("Dev_Hen", Some("Goldie".into())).unwrap();
    let interior = w.animals[0].home.clone();
    w.warp_farmer(&interior, TilePos::new(3, 3), Some(Dir::Down));
    let t = w.animals[0].tile();
    w.farmer.set_tile(TilePos::new(t.x, t.y - 1));
    w.farmer.facing = Dir::Down;
    hold(&mut w, Item::object("Dev_AnimalCracker", 2));
    assert!(w.do_action(Some(t)));
    assert!(w.animals[0].golden_cracker);
    assert_eq!(w.farmer.inventory.count("Dev_AnimalCracker"), 1);
    w.do_action(Some(w.animals[0].tile()));
    assert_eq!(w.farmer.inventory.count("Dev_AnimalCracker"), 1, "only one per animal");
    // Grow up, then count the eggs laid in one night.
    for _ in 0..4 {
        // Rain keeps her in; hay in the trough feeds her.
        w.weather = valley_sim::time::Weather::Rain;
        let loc = w.locations.get_mut(&interior).unwrap();
        loc.state.objects.retain(|_, o| o.item_id != "Dev_Egg");
        loc.state.objects.insert(TilePos::new(2, 2), PlacedObject::new("Dev_Hay", false, ObjKind::Placed));
        w.end_day(false);
    }
    let eggs = w.locations[&interior].state.objects.values().filter(|o| o.item_id == "Dev_Egg").count();
    assert_eq!(eggs, 2);
}

#[test]
fn treasure_totems_raise_artifact_spots_and_fairy_dust_rushes_machines() {
    let mut w = world();
    w.warp_farmer("Town", TilePos::new(10, 20), Some(Dir::Down));
    w.event = None;
    let spots = |w: &World| w.locations["Town"].state.objects.values().filter(|o| matches!(o.kind, ObjKind::ArtifactSpot)).count();
    let before = spots(&w);
    hold(&mut w, Item::object("Dev_TreasureTotem", 1));
    assert!(w.do_action(None));
    assert!(spots(&w) >= before + 4, "{} -> {}", before, spots(&w));
    assert_eq!(w.farmer.inventory.count("Dev_TreasureTotem"), 0);
    // A busy machine finishes at once.
    w.warp_farmer("Farm", TilePos::new(12, 18), Some(Dir::Down));
    let t = TilePos::new(12, 19);
    w.locations.get_mut("Farm").unwrap().state.objects.insert(t, PlacedObject::new("Dev_Juicer", true, ObjKind::Machine { output: Some(Item::object("Dev_Juice", 1)), minutes_left: 900, ready: false }));
    hold(&mut w, Item::object("Dev_FairyDust", 1));
    assert!(w.do_action(Some(t)));
    assert!(matches!(w.locations["Farm"].state.objects[&t].kind, ObjKind::Machine { ready: true, .. }));
}

#[test]
fn big_chests_hold_seventy_and_the_flute_calls_the_horse() {
    let mut w = world();
    w.warp_farmer("Farm", TilePos::new(12, 18), Some(Dir::Down));
    hold(&mut w, Item::big("Dev_BigChest", 1));
    assert!(w.begin_use(Some(TilePos::new(12, 19))));
    let Some(PlacedObject { kind: ObjKind::Chest(inv), .. }) = w.locations["Farm"].state.objects.get(&TilePos::new(12, 19)) else { panic!("no chest") };
    assert_eq!(inv.capacity(), 70);
    // The horse comes to the flute.
    w.place_building("Dev_Stable", TilePos::new(20, 12)).unwrap();
    w.end_day(false);
    w.event = None;
    w.warp_farmer("Town", TilePos::new(10, 20), Some(Dir::Down));
    w.event = None;
    hold(&mut w, Item::object("Dev_HorseFlute", 1));
    assert!(w.do_action(None));
    let h = w.buildings.iter().find_map(|b| b.horse.clone()).expect("horse");
    assert_eq!(h.location, "Town");
    assert_eq!(w.farmer.inventory.count("Dev_HorseFlute"), 1, "the flute is kept");
}
