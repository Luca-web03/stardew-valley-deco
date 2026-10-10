use valley_content::ContentManager;
use valley_sim::crop::Crop;
use valley_sim::item::Item;
use valley_sim::terrain::{HoeDirt, TerrainFeature};
use valley_sim::world::{NewGame, World, WorldEvent};
use valley_sim::{Dir, TilePos};

fn world() -> World {
    let mut cm = ContentManager::dev_only();
    World::new(&mut cm, NewGame::default())
}

fn plant_ready(w: &mut World, t: TilePos) {
    let seed = "Dev_TurnipSeeds";
    let data = w.items.crops[seed].clone();
    let mut crop = Crop::new(seed, &mut engine::Rng::new(1));
    crop.phase = data.days_in_phase.len();
    let mut dirt = HoeDirt::default();
    dirt.crop = Some(crop);
    w.locations.get_mut("Farm").unwrap().state.terrain.insert(t, TerrainFeature::HoeDirt(dirt));
}

#[test]
fn huts_gather_nearby_ripe_crops_into_storage() {
    let mut w = world();
    w.farmer.money = 50_000;
    w.farmer.inventory.add(Item::object("390", 300));
    w.farmer.inventory.add(Item::object("Dev_Berry", 20));
    {
        let farm = w.locations.get_mut("Farm").unwrap();
        for y in 16..30 {
            for x in 8..34 {
                farm.state.objects.remove(&TilePos::new(x, y));
                farm.state.terrain.remove(&TilePos::new(x, y));
            }
        }
    }
    w.place_building("Dev_HarvestHut", TilePos::new(12, 18)).unwrap();
    w.end_day(false);
    let near = [TilePos::new(10, 20), TilePos::new(15, 22), TilePos::new(18, 17)];
    let far = TilePos::new(30, 28);
    for t in near.iter().chain([far].iter()) {
        plant_ready(&mut w, *t);
    }
    w.end_day(false);
    let store = w.buildings[0].storage.clone().expect("hut storage");
    let turnips: i32 = store.slots.iter().flatten().filter(|i| i.object_id() == Some("Dev_Turnip")).map(|i| i.stack()).sum();
    assert!(turnips >= 3, "three nearby crops gathered ({turnips})");
    let farm = &w.locations["Farm"];
    assert!(near.iter().all(|t| !matches!(farm.terrain(*t), Some(TerrainFeature::HoeDirt(d)) if d.crop.is_some())));
    assert!(matches!(farm.terrain(far), Some(TerrainFeature::HoeDirt(d)) if d.crop.is_some()), "out of range");
    // The hut opens as a chest and items can be taken out.
    w.warp_farmer("Farm", TilePos::new(13, 20), Some(Dir::Up));
    w.drain_events();
    assert!(w.do_action(Some(TilePos::new(13, 19))));
    let tile = w.drain_events().into_iter().find_map(|e| match e {
        WorldEvent::OpenChest(t) => Some(t),
        _ => None,
    });
    let tile = tile.expect("hut storage opens");
    assert_eq!(tile, valley_sim::harvest_huts::hut_tile(w.buildings[0].id));
    let before = w.farmer.inventory.count("Dev_Turnip");
    let slot = w.container(tile).unwrap().slots.iter().position(|s| s.is_some()).unwrap();
    w.chest_transfer(tile, false, slot);
    assert!(w.farmer.inventory.count("Dev_Turnip") > before);
}
