use valley_content::ContentManager;
use valley_sim::item::{Item, ToolKind};
use valley_sim::objects::{ObjKind, PlacedObject};
use valley_sim::terrain::{Grass, TerrainFeature};
use valley_sim::time::Weather;
use valley_sim::world::{NewGame, World};
use valley_sim::TilePos;

fn world() -> World {
    let mut cm = ContentManager::dev_only();
    World::new(&mut cm, NewGame::default())
}

#[test]
fn new_farm_has_grass() {
    let w = world();
    let n = w.locations["Farm"].state.terrain.values().filter(|t| matches!(t, TerrainFeature::Grass(_))).count();
    assert!(n > 5, "{n}");
}

#[test]
fn scythe_hay_needs_a_silo() {
    let mut w = world();
    w.warp_farmer("Farm", TilePos::new(12, 18), None);
    let cut_many = |w: &mut World| {
        for i in 0..40 {
            let t = TilePos::new(10 + i % 8, 17 + i / 8);
            w.location_mut().state.objects.remove(&t);
            w.location_mut().state.terrain.insert(t, TerrainFeature::Grass(Grass { weeds: 2 }));
            w.apply_tool(ToolKind::Scythe, 4, t);
        }
    };
    cut_many(&mut w);
    assert_eq!(w.farmer.hay_storage, 0, "no silo, no hay");
    w.farmer.money = 1000;
    w.farmer.inventory.add(Item::object("390", 100));
    w.farmer.inventory.add(Item::object("Dev_CopperBar", 5));
    let farm = w.locations.get_mut("Farm").unwrap();
    for y in 2..6 {
        for x in 36..40 {
            farm.state.objects.remove(&TilePos::new(x, y));
            farm.state.terrain.remove(&TilePos::new(x, y));
        }
    }
    w.place_building("Silo", TilePos::new(37, 3)).unwrap();
    w.end_day(false);
    assert_eq!(w.silo_capacity(), 240);
    w.warp_farmer("Farm", TilePos::new(12, 16), None);
    cut_many(&mut w);
    assert!(w.farmer.hay_storage > 5, "{}", w.farmer.hay_storage);
    // The hay hopper hands it out one at a time.
    let before = w.farmer.hay_storage;
    assert!(w.hay_hopper_public());
    assert_eq!(w.farmer.hay_storage, before - 1);
    assert_eq!(w.farmer.inventory.count("Dev_Hay"), 1);
}

#[test]
fn lightning_charges_a_rod() {
    let mut w = world();
    let t = TilePos::new(14, 20);
    w.locations.get_mut("Farm").unwrap().state.objects.insert(t, PlacedObject::new("Dev_LightningRod", true, ObjKind::Placed));
    w.weather = Weather::Storm;
    let mut struck = false;
    for _ in 0..200 {
        w.update_ms(&Default::default(), valley_sim::time::TEN_MINUTES_MS);
        if matches!(w.locations["Farm"].object(t).unwrap().kind, ObjKind::Machine { output: Some(_), .. }) {
            struck = true;
            break;
        }
        if w.clock.time >= 2500 {
            break;
        }
    }
    assert!(struck, "a storm day should eventually hit the rod");
}
