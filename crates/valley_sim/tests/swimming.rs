use valley_content::ContentManager;
use valley_sim::world::{NewGame, World};
use valley_sim::{Dir, TilePos};

fn step(w: &mut World, dx: i32, dy: i32, ticks: u32) {
    let input = valley_sim::input::PlayerInput { move_x: dx, move_y: dy, ..Default::default() };
    for _ in 0..ticks {
        w.update_ms(&input, 16);
    }
}

#[test]
fn swimsuit_pool_and_spa() {
    let mut cm = ContentManager::dev_only();
    let mut w = World::new(&mut cm, NewGame::default());
    // Into the bath house from town.
    w.warp_farmer("Town", TilePos::new(37, 22), Some(Dir::Up));
    assert!(w.do_action(Some(TilePos::new(37, 21))));
    assert_eq!(w.current, "Dev_Spa");
    w.event = None;
    // Without the swimsuit the pool entrance refuses.
    w.farmer.set_tile(TilePos::new(5, 4));
    step(&mut w, 1, 0, 20);
    assert!(!w.farmer.swimming);
    assert!(w.farmer.tile().x <= 6);
    // Lockers: change, then hop in.
    w.farmer.set_tile(TilePos::new(3, 3));
    step(&mut w, 0, 0, 2);
    assert!(w.farmer.swimsuit);
    w.farmer.set_tile(TilePos::new(5, 4));
    w.farmer.facing = Dir::Right;
    step(&mut w, 1, 0, 20);
    assert!(w.farmer.swimming, "in the pool");
    assert!(w.location().is_water(w.farmer.tile()));
    // Swimming can't leave the water except through an entrance.
    w.farmer.set_tile(TilePos::new(10, 2));
    step(&mut w, 0, -1, 30);
    assert_eq!(w.farmer.tile().y, 2);
    // The spa restores energy.
    w.farmer.stamina = 10.0;
    step(&mut w, 0, 0, 60);
    assert!(w.farmer.stamina > 10.0);
    // Leaving the bath house ends the swim.
    w.warp_farmer("Town", TilePos::new(37, 22), None);
    assert!(!w.farmer.swimming);
}
