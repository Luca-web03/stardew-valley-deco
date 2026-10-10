use valley_content::ContentManager;
use valley_sim::input::PlayerInput;
use valley_sim::item::Item;
use valley_sim::save::SaveData;
use valley_sim::world::{NewGame, World};
use valley_sim::{Dir, TilePos};

fn world_with_stable() -> (ContentManager, World) {
    let mut cm = ContentManager::dev_only();
    let mut w = World::new(&mut cm, NewGame::default());
    w.farmer.money = 10_000;
    w.farmer.inventory.add(Item::object("388", 200));
    {
        let farm = w.locations.get_mut("Farm").unwrap();
        for y in 14..30 {
            for x in 8..40 {
                farm.state.objects.remove(&TilePos::new(x, y));
                farm.state.terrain.remove(&TilePos::new(x, y));
            }
        }
    }
    w.place_building("Dev_Stable", TilePos::new(10, 18)).unwrap();
    w.end_day(false);
    (cm, w)
}

fn walk(w: &mut World, dx: i32, ticks: u32) -> f32 {
    let x0 = w.farmer.pos.x;
    for _ in 0..ticks {
        w.update(&PlayerInput { move_x: dx, ..Default::default() });
    }
    w.farmer.pos.x - x0
}

#[test]
fn stable_comes_with_a_horse_that_is_faster() {
    let (_, mut w) = world_with_stable();
    let horse = w.buildings[0].horse.clone().expect("a horse once the stable is built");
    assert_eq!(horse.location, "Farm");
    let ht = TilePos::from_world(horse.pos + engine::Vec2::new(32.0, 48.0));
    assert_eq!(ht, TilePos::new(12, 20));
    // On foot first.
    w.warp_farmer("Farm", TilePos::new(14, 24), Some(Dir::Right));
    let foot = walk(&mut w, 1, 60);
    // Mount from beside the horse.
    w.warp_farmer("Farm", TilePos::new(12, 21), Some(Dir::Up));
    assert!(w.do_action(Some(TilePos::new(12, 20))));
    assert!(w.farmer.mounted.is_some());
    let riding = walk(&mut w, 1, 60);
    assert!(riding > foot * 1.4, "riding {riding} vs walking {foot}");
    // The horse moves with the farmer, tools are off-limits while riding.
    let hp = w.buildings[0].horse.as_ref().unwrap().pos;
    assert_eq!(hp, w.farmer.pos);
    assert!(!w.begin_use(None));
    // Dismount where we stand.
    assert!(w.do_action(None));
    assert!(w.farmer.mounted.is_none());
    assert_eq!(w.buildings[0].horse.as_ref().unwrap().pos, w.farmer.pos);
}

#[test]
fn horse_waits_outside_and_goes_home_overnight() {
    let (mut cm, mut w) = world_with_stable();
    w.warp_farmer("Farm", TilePos::new(12, 21), Some(Dir::Up));
    assert!(w.do_action(Some(TilePos::new(12, 20))));
    w.warp_farmer("Town", TilePos::new(5, 15), Some(Dir::Right));
    assert!(w.farmer.mounted.is_some(), "outdoors the horse comes along");
    assert_eq!(w.buildings[0].horse.as_ref().unwrap().location, "Town");
    let before = w.farmer.pos;
    w.warp_farmer("SeedShop", TilePos::new(6, 8), Some(Dir::Up));
    assert!(w.farmer.mounted.is_none(), "dismounted at the door");
    let h = w.buildings[0].horse.clone().unwrap();
    assert_eq!((h.location.as_str(), h.pos), ("Town", before));
    w.end_day(false);
    let h = w.buildings[0].horse.clone().unwrap();
    assert_eq!(h.location, "Farm");
    let json = SaveData::capture(&w).to_json();
    let r = SaveData::from_json(&json).unwrap().restore(&mut cm);
    assert!(r.buildings[0].horse.is_some());
}
