use std::sync::Arc;

use valley_content::ContentManager;
use valley_sim::item::Item;
use valley_sim::world::{NewGame, World};
use valley_sim::{Dir, TilePos};

fn world() -> World {
    let mut cm = ContentManager::dev_only();
    let mut w = World::new(&mut cm, NewGame::default());
    w.event = None;
    w
}

fn idx(w: &World, name: &str) -> usize {
    w.npcs.iter().position(|n| n.name == name).unwrap()
}

fn edit(w: &mut World, name: &str, f: impl FnOnce(&mut valley_content::data::CharacterData)) {
    let i = idx(w, name);
    let mut d = (*w.npcs[i].data).clone();
    f(&mut d);
    w.npcs[i].data = Arc::new(d);
}

/// Put `name` right above the farmer in town.
fn meet(w: &mut World, name: &str) {
    let me = TilePos::new(18, 12);
    w.warp_farmer("Town", me, Some(Dir::Up));
    w.event = None;
    let i = idx(w, name);
    w.npcs[i].route.clear();
    w.npcs[i].location = "Town".into();
    w.npcs[i].set_tile(TilePos::new(18, 11));
    w.farmer.set_tile(me);
    w.farmer.facing = Dir::Up;
}

#[test]
fn trash_diving_upsets_onlookers_unless_they_dont_mind() {
    let mut w = world();
    meet(&mut w, "Mara");
    w.farmer.friendship("Mara").points = 500;
    edit(&mut w, "Tobin", |d| d.dumpster_dive_friendship_effect = Some(5));
    let t = idx(&w, "Tobin");
    w.npcs[t].location = "Town".into();
    w.npcs[t].set_tile(TilePos::new(20, 12));
    w.farmer.friendship("Tobin").points = 100;
    // The can by the store is a couple of tiles away.
    w.farmer.set_tile(TilePos::new(19, 10));
    w.farmer.facing = Dir::Up;
    assert!(w.do_action(Some(TilePos::new(19, 9))));
    assert_eq!(w.farmer.friendships["Mara"].points, 475);
    assert_eq!(w.farmer.friendships["Tobin"].points, 105);
    assert!(w.npcs[idx(&w, "Mara")].emote.is_some());
}

#[test]
fn gifts_social_tab_and_calendar_follow_character_data() {
    let mut w = world();
    let mara = idx(&w, "Mara");
    assert_eq!(w.social_tab_name(&w.npcs[mara]).as_deref(), Some("???"), "unknown until met");
    edit(&mut w, "Mara", |d| d.can_receive_gifts = false);
    meet(&mut w, "Mara");
    w.farmer.inventory.slots[11] = Some(Item::object("Dev_Turnip", 1));
    w.farmer.current_slot = 11;
    w.do_action(None);
    assert_eq!(w.farmer.inventory.count("Dev_Turnip"), 1, "no gifts, just a chat");
    assert!(w.farmer.friendships["Mara"].talked_today);
    assert_eq!(w.social_tab_name(&w.npcs[mara]).as_deref(), Some("Mara"));
    edit(&mut w, "Tobin", |d| {
        d.social_tab = Some("HiddenAlways".into());
        d.calendar = Some("HiddenUntilMet".into());
        d.can_socialize = Some("FALSE".into());
    });
    let tobin = idx(&w, "Tobin");
    assert_eq!(w.social_tab_name(&w.npcs[tobin]), None);
    assert!(!w.calendar_shows(&w.npcs[tobin]));
    meet(&mut w, "Tobin");
    w.farmer.current_slot = 30;
    w.do_action(None);
    assert!(!w.farmer.friendships.get("Tobin").is_some_and(|f| f.talked_today), "can't socialize");
}
