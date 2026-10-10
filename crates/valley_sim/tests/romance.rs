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
