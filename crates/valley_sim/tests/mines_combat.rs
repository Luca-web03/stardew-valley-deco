use valley_content::ContentManager;
use valley_sim::combat::{Monster, Movement};
use valley_sim::input::PlayerInput;
use valley_sim::item::{Item, ToolKind};
use valley_sim::objects::ObjKind;
use valley_sim::world::{NewGame, World, WorldEvent};
use valley_sim::{Dir, TilePos};

fn world() -> World {
    let mut cm = ContentManager::dev_only();
    World::new(&mut cm, NewGame::default())
}

fn idle(w: &mut World, ticks: u32) {
    for _ in 0..ticks {
        w.update(&PlayerInput::default());
    }
}

#[test]
fn entering_the_mine_builds_a_level() {
    let mut w = world();
    w.warp_farmer("Mine", TilePos::new(5, 4), Some(Dir::Up));
    w.farmer.set_tile(TilePos::new(5, 4));
    w.farmer.facing = Dir::Up;
    assert!(w.do_action(None));
    assert_eq!(w.current, "UndergroundMine1");
    assert_eq!(w.mine_level(), Some(1));
    assert!(w.farmer.inventory.slots.iter().flatten().any(|i| i.weapon_id() == Some("Dev_Sword")));
    let loc = w.location();
    let rocks = loc.state.objects.values().filter(|o| matches!(o.kind, ObjKind::MineRock { .. })).count();
    assert!(rocks > 10, "{rocks}");
    assert!(loc.state.objects.values().any(|o| matches!(o.kind, ObjKind::Ladder { down: false })));
    assert!(w.monsters.iter().any(|m| m.location == "UndergroundMine1"));
    assert_eq!(w.farmer.deepest_mine_level, 1);
}

#[test]
fn breaking_all_rocks_reveals_a_ladder_and_descends() {
    let mut w = world();
    w.enter_mine_level(2);
    w.monsters.clear();
    let rocks: Vec<TilePos> = w.location().state.objects.iter().filter(|(_, o)| matches!(o.kind, ObjKind::MineRock { .. })).map(|(p, _)| *p).collect();
    let mut ladder = None;
    for p in rocks {
        while w.location().object(p).is_some_and(|o| matches!(o.kind, ObjKind::MineRock { .. })) {
            w.apply_tool(ToolKind::Pickaxe, 3, p);
        }
        if let Some(o) = w.location().object(p) {
            if matches!(o.kind, ObjKind::Ladder { down: true }) {
                ladder = Some(p);
                break;
            }
        }
    }
    let ladder = ladder.expect("a ladder appears before the rocks run out");
    idle(&mut w, 120);
    assert!(w.farmer.inventory.count("390") > 0 || w.dropped.iter().any(|d| d.item.object_id() == Some("390")));
    w.farmer.set_tile(TilePos::new(ladder.x, ladder.y + 1));
    w.farmer.facing = Dir::Up;
    // The tile below may be a wall; use the ladder directly.
    w.do_action(Some(ladder));
    assert_eq!(w.mine_level(), Some(3), "now at {}", w.current);
    assert!(!w.locations.contains_key("UndergroundMine2"), "old levels are unloaded");
}

#[test]
fn sword_kills_monster_and_monster_hurts_farmer() {
    let mut w = world();
    w.enter_mine_level(1);
    w.drain_events();
    w.monsters.clear();
    let me = w.farmer.tile();
    let info = w.monster_info["Dev_Blob"].clone();
    w.monsters.push(Monster::new(99, info, &w.current.clone(), TilePos::new(me.x, me.y + 1)));
    assert_eq!(w.monsters[0].movement, Movement::Hopper);
    let sword = (0..36).find(|&i| w.farmer.inventory.get(i).and_then(|x| x.weapon_id()).is_some()).unwrap();
    w.farmer.current_slot = sword;
    w.farmer.facing = Dir::Down;
    let health_before = w.farmer.health;
    let mut swings = 0;
    while !w.monsters.is_empty() && swings < 30 {
        w.farmer.facing = Dir::toward(w.monsters[0].tile().x - w.farmer.tile().x, w.monsters[0].tile().y - w.farmer.tile().y);
        w.update(&PlayerInput { use_tool: true, ..Default::default() });
        idle(&mut w, 30);
        swings += 1;
    }
    assert!(w.monsters.is_empty(), "monster survived {swings} swings");
    assert!(w.farmer.skills.xp(valley_sim::farmer::Skill::Combat) >= 3);
    assert_eq!(w.monsters_killed, 1);
    // A monster standing on the farmer deals contact damage once, then i-frames.
    let info = w.monster_info["Dev_Moth"].clone();
    let here = w.current.clone();
    let mut m = Monster::new(100, info, &here, w.farmer.tile());
    m.pos = w.farmer.pos;
    w.monsters.push(m);
    w.farmer.health = health_before;
    w.update(&PlayerInput::default());
    let after_hit = w.farmer.health;
    assert!(after_hit < health_before);
    w.update(&PlayerInput::default());
    assert_eq!(w.farmer.health, after_hit, "invincibility frames");
}

#[test]
fn fainting_sends_farmer_home_and_costs_money() {
    let mut w = world();
    w.enter_mine_level(4);
    w.drain_events();
    w.farmer.health = 1;
    w.farmer.money = 2000;
    let c = w.farmer.center();
    w.damage_farmer(50, c);
    assert_eq!(w.current, "FarmHouse");
    assert_eq!(w.farmer.money, 1800);
    assert!(w.farmer.health > 0);
    assert!(w.locations.keys().all(|k| !k.starts_with("UndergroundMine")));
    assert!(w.drain_events().iter().any(|e| matches!(e, WorldEvent::Message(_))));
}

#[test]
fn elevator_lists_floors() {
    let mut w = world();
    w.farmer.deepest_mine_level = 12;
    w.warp_farmer("Mine", TilePos::new(8, 3), Some(Dir::Up));
    w.farmer.set_tile(TilePos::new(8, 3));
    w.farmer.facing = Dir::Up;
    w.drain_events();
    assert!(w.do_action(None));
    assert!(w.drain_events().contains(&WorldEvent::ElevatorMenu { deepest: 12 }));
    let _ = Item::object("390", 1);
}
