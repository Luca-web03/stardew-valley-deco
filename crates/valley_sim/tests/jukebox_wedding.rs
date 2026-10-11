use valley_content::ContentManager;
use valley_sim::item::Item;
use valley_sim::world::{NewGame, World, WorldEvent};
use valley_sim::{Dir, TilePos};

fn world() -> World {
    let mut cm = ContentManager::dev_only();
    let mut w = World::new(&mut cm, NewGame::default());
    w.event = None;
    w
}

#[test]
fn mini_jukebox_plays_heard_songs_in_its_room() {
    let mut w = world();
    for cue in ["spring1", "rain", "Dev_Unreleased", "Dev_ThemeShort", "spring1", "Dev_Theme"] {
        w.note_song_heard(cue);
    }
    let list = w.jukebox_list();
    assert_eq!(list, vec![("spring1".to_string(), "Spring Fields".to_string()), ("Dev_Theme".to_string(), "Valley Theme".to_string())]);
    // Place one in the farmhouse and pick a track.
    w.farmer.set_tile(TilePos::new(4, 4));
    w.farmer.facing = Dir::Down;
    w.farmer.inventory.slots[11] = Some(Item::big("Dev_MiniJukebox", 1));
    w.farmer.current_slot = 11;
    assert!(w.begin_use(Some(TilePos::new(4, 5))));
    assert!(w.location().object(TilePos::new(4, 5)).is_some());
    w.farmer.current_slot = 30;
    w.drain_events();
    assert!(w.do_action(Some(TilePos::new(4, 5))));
    let ev = w.drain_events();
    let Some(WorldEvent::Choice { options, .. }) = ev.iter().find(|e| matches!(e, WorldEvent::Choice { .. })) else { panic!("{ev:?}") };
    assert_eq!(options.len(), 4, "off, random and two songs");
    w.choose("jukebox:Dev_Theme");
    assert_eq!(w.jukebox_track().as_deref(), Some("Dev_Theme"));
    w.choose("jukebox:random");
    assert!(w.jukebox_track().is_some());
    // Only where the jukebox is.
    w.warp_farmer("Farm", TilePos::new(12, 18), Some(Dir::Down));
    assert_eq!(w.jukebox_track(), None);
    w.warp_farmer("FarmHouse", TilePos::new(4, 4), Some(Dir::Down));
    w.choose("jukebox:off");
    assert_eq!(w.jukebox_track(), None);
}

#[test]
fn friends_call_the_telephone_once_and_it_stops_ringing() {
    let mut w = world();
    // Without a phone nobody calls (the starting one is taken away).
    for loc in w.locations.values_mut() {
        loc.state.objects.retain(|_, o| !(o.big && o.item_id == "Dev_Telephone"));
    }
    w.farmer.friendship("Mara").points = 600;
    w.end_day(false);
    assert!(w.pending_call.is_none());
    w.event = None;
    // Put a phone in the farmhouse.
    w.locations.get_mut("FarmHouse").unwrap().state.objects.insert(TilePos::new(4, 5), valley_sim::objects::PlacedObject::new("Dev_Telephone", true, valley_sim::objects::ObjKind::Placed));
    w.end_day(false);
    w.event = None;
    let p = w.pending_call.clone().expect("Mara's call ignores the base chance");
    assert_eq!(p.id, "Dev_MaraCall");
    assert!((900..=1700).contains(&p.ring_at));
    while w.clock.time < p.ring_at {
        w.clock.time = valley_sim::time::Clock::add_minutes(w.clock.time, 10);
        w.phone_calls_tick_pub();
    }
    assert!(w.phone_ringing());
    w.warp_farmer("FarmHouse", TilePos::new(4, 4), Some(Dir::Down));
    w.farmer.current_slot = 30;
    w.drain_events();
    assert!(w.do_action(Some(TilePos::new(4, 5))));
    let ev = w.drain_events();
    let Some(WorldEvent::Dialogue { npc, pages }) = ev.iter().find(|e| matches!(e, WorldEvent::Dialogue { .. })) else { panic!("{ev:?}") };
    assert_eq!(npc.as_deref(), Some("Mara"));
    assert!(pages[0].text.contains("checking the line"));
    assert!(!w.phone_ringing());
    // MaxCalls 1: never again.
    w.end_day(false);
    assert!(w.pending_call.as_ref().map_or(true, |p| p.id != "Dev_MaraCall"));
}
