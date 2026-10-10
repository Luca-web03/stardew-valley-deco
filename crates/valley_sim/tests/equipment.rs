use valley_content::ContentManager;
use valley_sim::combat::Monster;
use valley_sim::input::PlayerInput;
use valley_sim::item::Item;
use valley_sim::objects::ObjKind;
use valley_sim::world::{NewGame, World};
use valley_sim::{Dir, TilePos};

fn world() -> World {
    let mut cm = ContentManager::dev_only();
    World::new(&mut cm, NewGame::default())
}

#[test]
fn rings_and_boots_add_stats() {
    let mut w = world();
    assert_eq!(w.farmer_defense(), 0);
    w.farmer.inventory.slots[20] = Some(Item::object("Dev_GuardRing", 1));
    w.farmer.inventory.slots[21] = Some(Item::Boots { id: "Dev_WorkBoots".into() });
    w.farmer.inventory.slots[22] = Some(Item::object("Dev_MagnetRing", 1));
    assert!(w.equip(20));
    assert!(w.equip(21));
    assert!(w.equip(22));
    assert_eq!(w.farmer_defense(), 4, "ring 2 + boots 2");
    assert_eq!(w.total_attrs().magnetic_radius, 2);
    assert!(w.farmer.inventory.get(20).is_none());
    assert!(!w.equip(23), "nothing there");
    assert!(w.unequip(2));
    assert_eq!(w.farmer_defense(), 2);
    assert!(w.farmer.inventory.slots.iter().flatten().any(|i| matches!(i, Item::Boots { .. })));
}

#[test]
fn bomb_clears_rocks_and_hurts_monsters() {
    let mut w = world();
    w.enter_mine_level(2);
    w.drain_events();
    w.monsters.clear();
    let me = w.farmer.tile();
    // Find a rock cluster a few tiles away to blow up.
    let target = w.location().state.objects.iter().find(|(p, o)| matches!(o.kind, ObjKind::MineRock { .. }) && p.manhattan(me) >= 4).map(|(p, _)| *p).unwrap();
    let before = w.location().state.objects.iter().filter(|(p, o)| matches!(o.kind, ObjKind::MineRock { .. }) && (p.x - target.x).pow(2) + (p.y - target.y).pow(2) <= 9).count();
    assert!(before >= 1);
    let info = w.monster_info["Dev_Blob"].clone();
    let here = w.current.clone();
    w.monsters.push(Monster::new(5, info, &here, TilePos::new(target.x + 1, target.y)));
    w.location_mut().state.objects.remove(&target);
    // Light a radius-3 charge exactly at the target.
    w.bombs.push(valley_sim::equipment::LitBomb { location: here.clone(), tile: target, radius: 3, fuse_ms: 1500 });
    for _ in 0..120 {
        w.update(&PlayerInput::default());
    }
    let after = w.location().state.objects.iter().filter(|(p, o)| matches!(o.kind, ObjKind::MineRock { .. }) && (p.x - target.x).pow(2) + (p.y - target.y).pow(2) <= 9).count();
    assert_eq!(after, 0, "rocks in the blast radius are gone");
    assert!(w.monsters.iter().all(|m| m.health < m.info.health), "monster took damage");
}

#[test]
fn crab_pot_catches_overnight() {
    let mut w = world();
    w.warp_farmer("Farm", TilePos::new(40, 21), Some(Dir::Down));
    w.farmer.set_tile(TilePos::new(40, 21));
    w.farmer.facing = Dir::Down;
    let water = TilePos::new(40, 22);
    w.farmer.inventory.slots[11] = Some(Item::object("Dev_CrabPot", 1));
    w.farmer.current_slot = 11;
    assert!(w.do_action(None));
    assert!(matches!(w.location().object(water).unwrap().kind, ObjKind::CrabPot { bait: false, .. }));
    w.farmer.inventory.slots[11] = Some(Item::object("Dev_Bait", 1));
    assert!(w.do_action(None));
    w.end_day(false);
    let ObjKind::CrabPot { catch: Some(c), .. } = &w.locations["Farm"].object(water).unwrap().kind else { panic!("no catch") };
    let id = c.object_id().unwrap().to_string();
    assert!(id == "Dev_Crawler" || id == "Dev_Driftwood");
    w.warp_farmer("Farm", TilePos::new(40, 21), Some(Dir::Down));
    w.farmer.set_tile(TilePos::new(40, 21));
    w.farmer.facing = Dir::Down;
    w.farmer.current_slot = 30;
    assert!(w.do_action(None));
    assert_eq!(w.farmer.inventory.count(&id), 1);
}

#[test]
fn lighting_a_held_bomb_consumes_it() {
    let mut w = world();
    w.warp_farmer("Farm", TilePos::new(12, 18), Some(Dir::Down));
    w.farmer.set_tile(TilePos::new(12, 18));
    w.farmer.facing = Dir::Down;
    let t = TilePos::new(12, 19);
    w.location_mut().state.objects.remove(&t);
    w.location_mut().state.terrain.remove(&t);
    w.farmer.inventory.slots[11] = Some(Item::object("Dev_Bomb", 2));
    w.farmer.current_slot = 11;
    assert!(w.do_action(None));
    assert_eq!(w.bombs.len(), 1);
    assert_eq!(w.bombs[0].tile, t);
    assert_eq!(w.farmer.inventory.count("Dev_Bomb"), 1);
    let hp = w.farmer.health;
    for _ in 0..120 {
        w.update(&PlayerInput::default());
    }
    assert!(w.bombs.is_empty());
    assert!(w.farmer.health < hp, "standing next to a blast hurts");
}
