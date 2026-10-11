use engine::Vec2;
use valley_content::ContentManager;
use valley_sim::arsenal::Hit;
use valley_sim::combat::Monster;
use valley_sim::equipment::LitBomb;
use valley_sim::input::PlayerInput;
use valley_sim::monster_kinds::{species_for, ShotKind, Species};
use valley_sim::world::{NewGame, World};
use valley_sim::{TilePos, TILE};

fn world() -> World {
    let mut cm = ContentManager::dev_only();
    let mut w = World::new(&mut cm, NewGame::default());
    w.enter_mine_level(5);
    w.monsters.clear();
    w
}

/// Put a monster (stats from `base`, renamed) on an open tile near the farmer.
fn spawn(w: &mut World, base: &str, name: &str, dx: i32) -> u32 {
    let mut info = w.monster_info[base].clone();
    info.name = name.to_string();
    let me = w.farmer.tile();
    let loc = &w.locations[&w.current];
    let tile = (1..12)
        .flat_map(|k| [TilePos::new(me.x + dx * k, me.y), TilePos::new(me.x, me.y + dx * k)])
        .find(|t| valley_sim::collision::tile_passable(loc, *t) && t.manhattan(me) >= dx.unsigned_abs() as i32)
        .unwrap();
    w.next_monster_id += 1;
    let id = w.next_monster_id;
    let cur = w.current.clone();
    w.monsters.push(Monster::new(id, info, &cur, tile));
    id
}

fn monster(w: &World, id: u32) -> Option<&Monster> {
    w.monsters.iter().find(|m| m.id == id)
}

fn hit(w: &mut World, id: u32, dmg: i32) {
    let m = monster(w, id).unwrap();
    let b = m.bbox();
    w.hit_area(b, Hit { min: dmg, max: dmg, knockback: 0.0, crit: 0.0, crit_mult: 1.0, ignore_invincible: true });
}

fn tick(w: &mut World, n: u32) {
    for _ in 0..n {
        w.farmer.health = w.farmer.max_health;
        w.update(&PlayerInput::default());
    }
}

#[test]
fn names_pick_species() {
    assert_eq!(species_for("Rock Crab"), Species::Crab);
    assert_eq!(species_for("Magma Duggy"), Species::Burrower);
    assert_eq!(species_for("Skeleton"), Species::Shooter(ShotKind::Bone));
    assert_eq!(species_for("Skeleton Mage"), Species::Shooter(ShotKind::Spell));
    assert_eq!(species_for("Squid Kid"), Species::Shooter(ShotKind::Fireball));
    assert_eq!(species_for("Shadow Shaman"), Species::Shaman);
    assert_eq!(species_for("Mummy"), Species::Mummy);
    assert_eq!(species_for("Hot Head"), Species::HotHead);
    assert_eq!(species_for("Big Slime"), Species::BigSlime);
    assert_eq!(species_for("Grub"), Species::Grub);
    assert_eq!(species_for("Dust Spirit"), Species::Dust);
    assert_eq!(species_for("Green Slime"), Species::Normal);
}

#[test]
fn crabs_hide_as_rocks_and_shrug_off_the_first_hits() {
    let mut w = world();
    let id = spawn(&mut w, "Dev_StoneCrab", "Dev_StoneCrab", 3);
    assert!(monster(&w, id).unwrap().hidden);
    let start = monster(&w, id).unwrap().pos;
    tick(&mut w, 30);
    assert_eq!(monster(&w, id).unwrap().pos, start, "a disguised crab doesn't move");
    // Walk up to it.
    let p = monster(&w, id).unwrap().pos;
    w.farmer.pos = p + Vec2::new(0.0, TILE as f32);
    tick(&mut w, 2);
    assert!(!monster(&w, id).unwrap().hidden, "wakes up when the farmer is close");
    let hp = monster(&w, id).unwrap().health;
    for _ in 0..3 {
        hit(&mut w, id, 5);
    }
    assert_eq!(monster(&w, id).unwrap().health, hp, "the shell takes three hits");
    hit(&mut w, id, 5);
    assert!(monster(&w, id).unwrap().health < hp);
}

#[test]
fn burrowers_surface_beside_the_farmer() {
    let mut w = world();
    let id = spawn(&mut w, "Dev_Mole", "Dev_Mole", 4);
    let mut surfaced_near = false;
    let mut went_under = false;
    for _ in 0..600 {
        tick(&mut w, 1);
        let m = monster(&w, id).unwrap();
        went_under |= m.hidden;
        if !m.hidden && m.center().distance(w.farmer.center()) < 2.5 * TILE as f32 {
            surfaced_near = true;
        }
    }
    assert!(went_under && surfaced_near);
}

#[test]
fn skeletons_throw_bones_that_hurt() {
    let mut w = world();
    spawn(&mut w, "Dev_Skeleton", "Dev_Skeleton", 4);
    let mut saw_shot = false;
    let mut hurt = false;
    for _ in 0..600 {
        let hp = w.farmer.health;
        w.update(&PlayerInput::default());
        saw_shot |= !w.enemy_shots.is_empty();
        if w.farmer.health < hp {
            hurt = true;
        }
        w.farmer.health = w.farmer.max_health;
        w.farmer.invincible_ms = 0;
    }
    assert!(saw_shot, "bones were thrown");
    assert!(hurt);
}

#[test]
fn mummies_get_back_up_unless_bombed() {
    let mut w = world();
    let id = spawn(&mut w, "Dev_Mummy", "Dev_Mummy", 3);
    hit(&mut w, id, 500);
    let m = monster(&w, id).expect("collapsed, not dead");
    assert!(m.collapsed_ms > 0);
    tick(&mut w, 60 * 11);
    let m = monster(&w, id).expect("still here");
    assert_eq!(m.collapsed_ms, 0);
    assert_eq!(m.health, m.info.health, "back on its feet");
    // A bomb ends it for good.
    hit(&mut w, id, 500);
    let t = TilePos::from_world(monster(&w, id).unwrap().center());
    let cur = w.current.clone();
    w.bombs.push(LitBomb { location: cur, tile: t, radius: 3, fuse_ms: 0 });
    tick(&mut w, 2);
    assert!(monster(&w, id).is_none());
}

#[test]
fn hot_heads_explode_and_big_slimes_split() {
    let mut w = world();
    let id = spawn(&mut w, "Dev_Blob", "Hot Head", 3);
    hit(&mut w, id, 1);
    assert!(monster(&w, id).unwrap().fuse_ms.is_some());
    tick(&mut w, 120);
    assert!(monster(&w, id).is_none(), "blew up");

    let id = spawn(&mut w, "Dev_Blob", "Big Blob", 3);
    let before = w.monsters.len();
    hit(&mut w, id, 500);
    assert!(monster(&w, id).is_none());
    assert!(w.monsters.len() >= before + 1, "split into smaller blobs");
}

#[test]
fn grubs_turn_into_flies() {
    let mut w = world();
    let mut fly = w.monster_info["Dev_Moth"].clone();
    fly.name = "Test Fly".into();
    w.monster_info.insert("Test Fly".into(), fly);
    let id = spawn(&mut w, "Dev_Larva", "Dev_Larva", 6);
    tick(&mut w, 60 * 13);
    assert!(monster(&w, id).is_none());
    assert!(w.monsters.iter().any(|m| m.info.name == "Test Fly"));
}
