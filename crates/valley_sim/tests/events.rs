use valley_content::ContentManager;
use valley_sim::input::PlayerInput;
use valley_sim::world::{NewGame, World, WorldEvent};
use valley_sim::{Dir, TilePos};

fn world() -> World {
    let mut cm = ContentManager::dev_only();
    World::new(&mut cm, NewGame::default())
}

#[test]
fn intro_event_plays_once_on_entering_town() {
    let mut w = world();
    w.warp_farmer("Town", TilePos::new(2, 15), Some(Dir::Right));
    assert!(w.in_event());
    let mut dialogues = 0;
    for _ in 0..(60 * 60) {
        w.update(&PlayerInput::default());
        for e in w.drain_events() {
            if let WorldEvent::Dialogue { npc, pages } = e {
                assert_eq!(npc.as_deref(), Some("Mara"));
                assert!(pages[0].text.contains("new farmer"));
                assert_eq!(pages.len(), 2);
                dialogues += 1;
                // The UI would show the dialogue; simulate closing it.
                w.event_resume();
            }
        }
        if !w.in_event() {
            break;
        }
    }
    assert!(!w.in_event(), "event finished");
    assert_eq!(dialogues, 1);
    assert!(w.farmer.events_seen.contains(&"Dev_Intro".to_string()));
    assert_eq!(w.farmer.friendships["Mara"].points, 50);
    // Doesn't repeat.
    w.warp_farmer("Farm", TilePos::new(20, 12), None);
    w.warp_farmer("Town", TilePos::new(2, 15), None);
    assert!(!w.in_event());
}

#[test]
fn time_precondition_blocks_event() {
    let mut w = world();
    w.clock.time = 2000;
    w.warp_farmer("Town", TilePos::new(2, 15), Some(Dir::Right));
    assert!(!w.in_event());
    assert!(w.event_preconditions("x/t 1900 2100"));
    assert!(!w.event_preconditions("x/f Mara 100"));
    assert!(!w.event_preconditions("x/z spring"));
    assert!(w.event_preconditions("x/z winter"));
}

#[test]
fn skip_ends_event_immediately() {
    let mut w = world();
    w.warp_farmer("Town", TilePos::new(2, 15), Some(Dir::Right));
    for _ in 0..5 {
        w.update(&PlayerInput::default());
    }
    w.skip_event();
    assert!(!w.in_event());
    assert!(w.farmer.events_seen.contains(&"Dev_Intro".to_string()));
}
