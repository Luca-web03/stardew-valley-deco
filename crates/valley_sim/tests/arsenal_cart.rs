use valley_content::ContentManager;
use valley_sim::combat::Monster;
use valley_sim::input::PlayerInput;
use valley_sim::item::Item;
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

/// Farmer on open farm ground holding `weapon`, facing right.
fn armed(weapon: &str) -> World {
    let mut w = world();
    let me = TilePos::new(14, 12);
    {
        let farm = w.locations.get_mut("Farm").unwrap();
        for x in 10..24 {
            for y in 9..16 {
                farm.state.objects.remove(&TilePos::new(x, y));
                farm.state.terrain.remove(&TilePos::new(x, y));
            }
        }
    }
    w.warp_farmer("Farm", me, Some(Dir::Right));
    w.farmer.set_tile(me);
    w.farmer.facing = Dir::Right;
    w.farmer.inventory.slots[5] = Some(Item::Weapon { id: weapon.into() });
    w.farmer.current_slot = 5;
    w.drain_events();
    w
}

/// A tough, stationary-ish blob at `t` on the current map.
fn blob(w: &mut World, id: u32, t: TilePos) -> usize {
    let info = w.monster_info["Dev_Blob"].clone();
    let here = w.current.clone();
    let mut m = Monster::new(id, info, &here, t);
    m.health = 1000;
    m.info.miss_chance = 0.0;
    m.pos = engine::Vec2::new(t.x as f32 * 64.0, t.y as f32 * 64.0 - 16.0);
    w.monsters.push(m);
    w.monsters.len() - 1
}

#[test]
fn dagger_flurry_strikes_several_times() {
    let mut w = armed("Dev_Dagger");
    let i = blob(&mut w, 1, TilePos::new(15, 12));
    assert!(w.begin_special_move());
    assert!(w.special.is_some());
    idle(&mut w, 40);
    assert!(w.special.is_none());
    let hits = w.floaters.iter().filter(|f| f.text.parse::<i32>().is_ok()).count();
    assert!(hits >= 3, "flurry hits: {hits}");
    assert!(w.monsters[i].health < 1000);
}

#[test]
fn club_slam_hits_all_around_after_wind_up() {
    let mut w = armed("Dev_Club");
    let a = blob(&mut w, 1, TilePos::new(14, 10));
    let b = blob(&mut w, 2, TilePos::new(12, 12));
    assert!(w.begin_special_move());
    w.update(&PlayerInput::default());
    assert_eq!(w.monsters[a].health, 1000, "nothing lands during the wind-up");
    idle(&mut w, 40);
    assert!(w.monsters[a].health < 1000, "behind the farmer");
    assert!(w.monsters[b].health < 1000, "to the side");
}

#[test]
fn sword_block_prevents_contact_damage_and_has_a_cooldown() {
    let mut w = armed("Dev_Sword");
    let before = w.farmer.health;
    assert!(w.begin_special_move());
    assert!(w.is_blocking());
    let info = w.monster_info["Dev_Moth"].clone();
    let here = w.current.clone();
    let mut m = Monster::new(9, info, &here, w.farmer.tile());
    m.pos = w.farmer.pos;
    w.monsters.push(m);
    w.update(&PlayerInput::default());
    assert_eq!(w.farmer.health, before, "blocked");
    idle(&mut w, 60);
    assert!(!w.is_blocking());
    assert!(w.special_cooldown_ms > 0);
    w.monsters.clear();
    assert!(w.begin_special_move(), "the press is consumed");
    assert!(w.special.is_none(), "but nothing starts while cooling down");
    idle(&mut w, 60 * 4);
    assert!(w.begin_special_move());
    assert!(w.is_blocking());
}

#[test]
fn action_button_with_a_weapon_starts_the_special() {
    let mut w = armed("Dev_Sword");
    assert!(w.do_action(Some(TilePos::new(15, 12))));
    assert!(w.is_blocking());
}

#[test]
fn slingshot_fires_ammo_at_range() {
    let mut w = armed("Dev_Slingshot");
    w.farmer.inventory.slots[6] = Some(Item::object("390", 10));
    let target = TilePos::new(19, 12);
    let i = blob(&mut w, 1, target);
    w.monsters[i].info.speed = 0.0;
    // Press, hold to pull, release.
    w.update(&PlayerInput { use_tool: true, use_held: true, cursor_tile: Some(target), ..Default::default() });
    assert!(w.sling.is_some());
    for _ in 0..40 {
        w.update(&PlayerInput { use_held: true, cursor_tile: Some(target), ..Default::default() });
    }
    w.update(&PlayerInput { cursor_tile: Some(target), ..Default::default() });
    assert!(w.sling.is_none());
    assert_eq!(w.farmer.inventory.count("390"), 9, "one stone used");
    assert_eq!(w.projectiles.len(), 1);
    idle(&mut w, 60);
    assert!(w.projectiles.is_empty());
    assert!(w.monsters[0].health < 1000, "the shot landed");
}

#[test]
fn slingshot_without_ammo_does_nothing() {
    let mut w = armed("Dev_Slingshot");
    for s in w.farmer.inventory.slots.iter_mut() {
        if matches!(s, Some(Item::Object { id, .. }) if w_ammo(id)) {
            *s = None;
        }
    }
    w.update(&PlayerInput { use_tool: true, use_held: true, ..Default::default() });
    assert!(w.sling.is_none());
    assert!(w.projectiles.is_empty());
}

fn w_ammo(id: &str) -> bool {
    matches!(id, "390" | "388" | "378" | "380" | "382" | "384" | "386") || id.ends_with("Ore") || id == "Dev_Coal"
}

/// Advance to the next morning that is `weekday`.
fn sleep_until(w: &mut World, weekday: &str) {
    for _ in 0..8 {
        if w.clock.weekday() == weekday {
            return;
        }
        w.end_day(false);
    }
    panic!("never reached {weekday}");
}

#[test]
fn cart_visits_on_fridays_and_sundays() {
    let mut w = world();
    assert_eq!(w.clock.weekday(), "Mon");
    assert!(w.cart_here("Town").is_none());
    sleep_until(&mut w, "Fri");
    let spot = w.cart_here("Town").expect("cart parked in town on Friday");
    assert_eq!(spot, TilePos::new(28, 18));
    assert!(w.locations["Town"].blocked.contains(&TilePos::new(29, 19)), "the wagon blocks its bottom row");
    w.end_day(false);
    assert_eq!(w.clock.weekday(), "Sat");
    assert!(w.cart_here("Town").is_none());
    assert!(!w.locations["Town"].blocked.contains(&TilePos::new(29, 19)));
    w.end_day(false);
    assert!(w.cart_here("Town").is_some(), "back on Sunday");
}

#[test]
fn cart_sells_random_one_off_stock() {
    let mut w = world();
    sleep_until(&mut w, "Fri");
    w.warp_farmer("Town", TilePos::new(29, 20), Some(Dir::Up));
    w.drain_events();
    assert!(w.do_action(Some(TilePos::new(29, 19))));
    let stock = w
        .drain_events()
        .into_iter()
        .find_map(|e| match e {
            WorldEvent::OpenShop(s) => Some(s),
            _ => None,
        })
        .expect("the cart opens a shop");
    assert_eq!(stock.id, "Traveler");
    assert_eq!(stock.entries.len(), 7, "six random items plus one pick");
    assert!(stock.entries.iter().all(|e| e.stock == 1 && e.price > 0));
    // Prices are random-sale prices: at least 100g each.
    assert!(stock.entries.iter().all(|e| e.price >= 100));
    let entry = stock.entries[0].clone();
    w.farmer.money = 100_000;
    assert!(w.buy_from("Traveler", &entry, 5));
    assert_eq!(w.farmer.money, 100_000 - entry.price, "one-off stock sells a single item");
    let again = w.cart_stock();
    assert_eq!(again.entries.len(), 6, "sold out for today");
    assert!(!w.buy_from("Traveler", &entry, 1), "a stale entry can't be bought twice");
    // The same day gives the same stock.
    let names: Vec<String> = again.entries.iter().map(|e| e.item.qualified_id()).collect();
    let names2: Vec<String> = w.cart_stock().entries.iter().map(|e| e.item.qualified_id()).collect();
    assert_eq!(names, names2);
}
