use valley_content::ContentManager;
use valley_sim::farmer::RelationStatus;
use valley_sim::item::Item;
use valley_sim::world::{NewGame, World, WorldEvent};
use valley_sim::{Dir, TilePos};

fn face_mara(w: &mut World) {
    let i = w.npcs.iter().position(|n| n.name == "Mara").unwrap();
    let home = w.npcs[i].home.tile;
    let loc = w.npcs[i].home.location.clone();
    w.warp_farmer(&loc, TilePos::new(home.x, home.y + 1), Some(Dir::Up));
    w.event = None;
    w.npcs[i].route.clear();
    w.npcs[i].location = loc;
    w.npcs[i].set_tile(home);
    w.farmer.set_tile(TilePos::new(home.x, home.y + 1));
    w.farmer.facing = Dir::Up;
}

fn hold(w: &mut World, id: &str) {
    w.farmer.inventory.slots[11] = Some(Item::object(id, 1));
    w.farmer.current_slot = 11;
}

#[test]
fn hearts_cap_until_dating_then_engagement_and_wedding() {
    let mut cm = ContentManager::dev_only();
    let mut w = World::new(&mut cm, NewGame::default());
    w.farmer.events_seen.push("Dev_Intro".into());
    w.farmer.friendship("Mara").points = 1990;
    face_mara(&mut w);
    w.farmer.current_slot = 30;
    w.do_action(None);
    assert_eq!(w.farmer.friendships["Mara"].points, 2000, "capped at 8 hearts");

    hold(&mut w, "Dev_Pendant");
    w.drain_events();
    w.do_action(None);
    assert_eq!(w.farmer.friendships["Mara"].status, RelationStatus::Friendly, "not dating yet");
    assert_eq!(w.farmer.inventory.count("Dev_Pendant"), 1);

    hold(&mut w, "Dev_Bouquet");
    w.do_action(None);
    assert_eq!(w.farmer.friendships["Mara"].status, RelationStatus::Dating);
    assert_eq!(w.farmer.inventory.count("Dev_Bouquet"), 0);

    w.farmer.friendship("Mara").points = 2500;
    hold(&mut w, "Dev_Pendant");
    w.do_action(None);
    assert!(matches!(w.farmer.friendships["Mara"].status, RelationStatus::Engaged { .. }));

    for _ in 0..3 {
        w.end_day(false);
    }
    // The ceremony plays in town (Data/Weddings), then it's home.
    let e = w.event.as_ref().expect("wedding event");
    assert_eq!((e.id.as_str(), e.location.as_str()), ("Port.Wedding", "Town"));
    assert!(e.actors.iter().any(|a| a.name == "Mara"));
    assert!(e.actors.iter().any(|a| a.name == "Tobin"), "attendees set up");
    assert!(!e.commands.iter().any(|c| c.contains('[')), "tokens replaced: {:?}", e.commands);
    w.skip_event();
    assert!(w.event.is_none());
    assert_eq!(w.current, "FarmHouse");
    assert_eq!(w.farmer.spouse.as_deref(), Some("Mara"));
    assert_eq!(w.farmer.friendships["Mara"].status, RelationStatus::Married);
    let mara = w.npcs.iter().find(|n| n.name == "Mara").unwrap();
    assert_eq!(mara.location, "FarmHouse");

    // Spouse greeting comes from the marriage dialogue table.
    let i = w.npcs.iter().position(|n| n.name == "Mara").unwrap();
    let t = w.npcs[i].tile();
    w.farmer.set_tile(TilePos::new(t.x, t.y + 1));
    w.farmer.facing = Dir::Up;
    w.drain_events();
    w.do_action(None);
    let ev = w.drain_events();
    let Some(WorldEvent::Dialogue { pages, .. }) = ev.iter().find(|e| matches!(e, WorldEvent::Dialogue { .. })) else { panic!("{ev:?}") };
    assert!(pages[0].text.contains("tea") || pages[0].text.contains("rain") || pages[0].text.contains("shelves"), "{}", pages[0].text);
}

#[test]
fn non_romanceable_declines_bouquet() {
    let mut cm = ContentManager::dev_only();
    let mut w = World::new(&mut cm, NewGame::default());
    let i = w.npcs.iter().position(|n| n.name == "Tobin").unwrap();
    let home = w.npcs[i].home.tile;
    w.warp_farmer("SeedShop", TilePos::new(home.x, home.y + 1), Some(Dir::Up));
    w.npcs[i].route.clear();
    w.npcs[i].location = "SeedShop".into();
    w.npcs[i].set_tile(home);
    w.farmer.set_tile(TilePos::new(home.x, home.y + 1));
    w.farmer.facing = Dir::Up;
    w.farmer.friendship("Tobin").points = 2500;
    hold(&mut w, "Dev_Bouquet");
    w.do_action(None);
    assert_eq!(w.farmer.friendships["Tobin"].status, RelationStatus::Friendly);
    assert_eq!(w.farmer.inventory.count("Dev_Bouquet"), 1);
}

#[test]
fn gifts_follow_quality_weekly_limits_and_friends_drift_apart() {
    let mut cm = ContentManager::dev_only();
    let mut w = World::new(&mut cm, NewGame::default());
    face_mara(&mut w);
    // Holding something gives it straight away (neutral: +20).
    hold(&mut w, "Dev_Turnip");
    w.do_action(None);
    assert_eq!(w.farmer.friendships["Mara"].points, 20);
    assert_eq!(w.farmer.inventory.count("Dev_Turnip"), 0);
    // Once a day: the next click talks instead.
    hold(&mut w, "Dev_Turnip");
    w.do_action(None);
    assert_eq!(w.farmer.inventory.count("Dev_Turnip"), 1);
    // Iridium quality: x1.5.
    w.end_day(false);
    face_mara(&mut w);
    w.farmer.inventory.slots[11] = Some(Item::object_q("Dev_Turnip", 1, valley_sim::item::Quality::Iridium));
    let before = w.farmer.friendships["Mara"].points;
    w.do_action(None);
    assert_eq!(w.farmer.friendships["Mara"].points, before + 30);
    // Two a week...
    w.farmer.friendship("Mara").gifts_this_week = 2;
    w.farmer.friendship("Mara").gifted_today = false;
    hold(&mut w, "Dev_Turnip");
    w.do_action(None);
    assert_eq!(w.farmer.inventory.count("Dev_Turnip"), 1, "weekly limit");
    // ...except on a birthday (spring 9), worth eight times as much.
    w.clock.day = 9;
    let before = w.farmer.friendships["Mara"].points;
    w.do_action(None);
    assert_eq!(w.farmer.friendships["Mara"].points, before + 160);
    // Friends not talked to lose 2 a day, but not once maxed; partners lose 8.
    w.farmer.friendship("Mara").points = 1000;
    w.farmer.friendship("Mara").talked_today = false;
    let mut days = 0;
    while days < 1 {
        w.end_day(false);
        days += 1;
    }
    assert_eq!(w.farmer.friendships["Mara"].points, 998);
    w.farmer.friendship("Mara").points = 2000;
    w.end_day(false);
    assert_eq!(w.farmer.friendships["Mara"].points, 2000, "maxed at 8 hearts");
    w.farmer.friendship("Mara").status = RelationStatus::Dating;
    w.end_day(false);
    assert_eq!(w.farmer.friendships["Mara"].points, 1992);
}

#[test]
fn a_happy_spouse_does_morning_chores() {
    let mut cm = ContentManager::dev_only();
    let mut w = World::new(&mut cm, NewGame::default());
    w.event = None;
    w.farmer.spouse = Some("Mara".into());
    w.farmer.friendship("Mara").status = RelationStatus::Married;
    w.farmer.friendship("Mara").points = 1000;
    assert_eq!(w.spouse_chores_pub(), None, "not happy enough");
    w.farmer.friendship("Mara").points = 3000;
    let mut seen = std::collections::BTreeSet::new();
    for day in 1..=28 {
        w.clock.day = day;
        w.weather = valley_sim::time::Weather::Sun;
        if let Some(p) = w.pet.as_mut() {
            p.bowl_filled = false;
        }
        let dishes_before: i32 = w.farmer.inventory.slots.iter().flatten().filter(|i| i.object_id().is_some_and(|id| w.items.object(id).is_some_and(|o| o.data.category == -7))).map(|i| i.stack()).sum();
        match w.spouse_chores_pub() {
            Some(c @ valley_sim::romance::Chore::PetBowl) => {
                assert!(w.pet.as_ref().unwrap().bowl_filled);
                seen.insert(format!("{c:?}"));
            }
            Some(c @ valley_sim::romance::Chore::Breakfast) => {
                let after: i32 = w.farmer.inventory.slots.iter().flatten().filter(|i| i.object_id().is_some_and(|id| w.items.object(id).is_some_and(|o| o.data.category == -7))).map(|i| i.stack()).sum();
                assert_eq!(after, dishes_before + 1);
                seen.insert(format!("{c:?}"));
            }
            Some(c) => {
                seen.insert(format!("{c:?}"));
            }
            None => {}
        }
    }
    assert!(seen.len() >= 2, "chores over a month: {seen:?}");
    let i = w.npcs.iter().position(|n| n.name == "Mara").unwrap();
    assert!(w.npcs[i].dialogue.get("__spouse_today").is_some());
}

#[test]
fn spouses_follow_marriage_schedules_on_dry_days() {
    let mut cm = ContentManager::dev_only();
    let mut w = World::new(&mut cm, NewGame::default());
    w.event = None;
    w.farmer.spouse = Some("Mara".into());
    w.farmer.friendship("Mara").status = RelationStatus::Married;
    w.farmer.friendship("Mara").points = 2600;
    let mut s = std::collections::HashMap::new();
    s.insert("marriage_Tue".to_string(), "900 Town 10 10 2".to_string());
    w.schedule_data.insert("Mara".into(), s);
    w.clock.day = 1;
    w.weather_tomorrow = valley_sim::time::Weather::Sun;
    w.end_day(false);
    assert_eq!(w.clock.weekday(), "Tue");
    let mara = w.npcs.iter().find(|n| n.name == "Mara").unwrap();
    assert_eq!(mara.location, "FarmHouse");
    assert_eq!(mara.schedule.first().map(|e| e.location.as_str()), Some("Town"));
    // Wednesday: no schedule, she stays home.
    w.weather_tomorrow = valley_sim::time::Weather::Sun;
    w.end_day(false);
    assert!(w.npcs.iter().find(|n| n.name == "Mara").unwrap().schedule.is_empty());
}
