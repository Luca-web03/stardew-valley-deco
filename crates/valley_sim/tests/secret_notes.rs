use valley_content::ContentManager;
use valley_sim::item::Item;
use valley_sim::objects::{ObjKind, PlacedObject};
use valley_sim::world::{NewGame, World, WorldEvent};
use valley_sim::{Dir, TilePos};

fn world() -> World {
    let mut cm = ContentManager::dev_only();
    World::new(&mut cm, NewGame::default())
}

#[test]
fn reading_scraps_reveals_notes_in_order() {
    let mut w = world();
    assert_eq!(w.note_item(), Some("Dev_NoteScrap"));
    assert_eq!(w.unread_notes(), vec!["1", "2", "3"]);
    w.farmer.inventory.slots[6] = Some(Item::object("Dev_NoteScrap", 2));
    w.farmer.current_slot = 6;
    w.drain_events();
    assert!(w.do_action(None));
    let msgs: Vec<String> = w.drain_events().into_iter().filter_map(|e| match e {
        WorldEvent::Message(m) => Some(m),
        _ => None,
    }).collect();
    assert!(msgs.iter().any(|m| m.contains("hums at night")));
    assert_eq!(w.farmer.notes_read, vec!["1"]);
    assert!(w.do_action(None));
    assert_eq!(w.farmer.notes_read, vec!["1", "2"]);
    assert!(w.drain_events().iter().any(|e| matches!(e, WorldEvent::Message(m) if m.contains("hollow.\nBring a pick"))));
    assert!(w.farmer.inventory.get(6).is_none());
}

#[test]
fn scraps_turn_up_while_digging_until_every_note_is_found() {
    let mut w = world();
    w.warp_farmer("Farm", TilePos::new(14, 12), Some(Dir::Down));
    let mut found = 0;
    for i in 0..300 {
        let t = TilePos::new(14, 13);
        w.location_mut().state.objects.insert(t, PlacedObject::new("Dev_ArtifactSpot", false, ObjKind::ArtifactSpot));
        w.game_ms += 17 * (i + 1);
        w.dig_artifact_spot(t);
        let scraps = w.dropped.iter().filter(|d| d.item.object_id() == Some("Dev_NoteScrap")).count();
        found = found.max(scraps);
    }
    assert!(found >= 1, "a scrap shows up eventually");
    assert!(found <= 3, "never more scraps than unread notes");
}
