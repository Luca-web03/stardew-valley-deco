use valley_content::ContentManager;
use valley_sim::input::PlayerInput;
use valley_sim::time::Season;
use valley_sim::world::{NewGame, World, WorldEvent};
use valley_sim::{Dir, TilePos};

fn run_until_roaming(w: &mut World) {
    for _ in 0..600 {
        w.update(&PlayerInput::default());
        for e in w.drain_events() {
            if matches!(e, WorldEvent::Dialogue { .. } | WorldEvent::Message(_)) {
                w.event_resume();
            }
        }
        if w.festival_roaming() {
            return;
        }
    }
    panic!("festival set-up never finished");
}

#[test]
fn harvest_fair_runs_and_ends_the_day() {
    let mut cm = ContentManager::dev_only();
    let mut w = World::new(&mut cm, NewGame::default());
    w.clock.season = Season::Fall;
    w.clock.day = 16;
    w.clock.time = 1000;
    w.farmer.events_seen.push("Dev_Intro".into());
    let money = w.farmer.money;
    w.warp_farmer("Town", TilePos::new(2, 15), Some(Dir::Right));
    assert!(w.in_event());
    run_until_roaming(&mut w);

    // Walk right toward the host (Tobin at 20,13) and talk to people on the way.
    let mut host_prompt = false;
    let mut chatted = false;
    for _ in 0..2000 {
        let farmer = w.event.as_ref().unwrap().actors.iter().find(|a| a.name == "farmer").unwrap().pos;
        let tx = 19.0 * 64.0;
        let ty = 13.0 * 64.0 - 16.0;
        let input = PlayerInput {
            move_x: if farmer.x < tx - 4.0 { 1 } else if farmer.x > tx + 4.0 { -1 } else { 0 },
            move_y: if (farmer.x - tx).abs() <= 4.0 { if farmer.y > ty + 4.0 { -1 } else if farmer.y < ty - 4.0 { 1 } else { 0 } } else { 0 },
            action: true,
            ..Default::default()
        };
        w.update(&input);
        for e in w.drain_events() {
            match e {
                WorldEvent::Dialogue { npc, .. } => {
                    chatted |= npc.as_deref() == Some("Mara");
                }
                WorldEvent::FestivalHost { .. } => host_prompt = true,
                _ => {}
            }
        }
        if host_prompt {
            break;
        }
        // Face right at the host's column.
        if (farmer.x - tx).abs() <= 4.0 && (farmer.y - ty).abs() <= 4.0 {
            w.event.as_mut().unwrap().actors.iter_mut().find(|a| a.name == "farmer").unwrap().facing = Dir::Right;
        }
    }
    assert!(host_prompt, "host asked to start the main event");
    let _ = chatted;
    w.festival_begin_main();
    for _ in 0..2000 {
        w.update(&PlayerInput::default());
        for e in w.drain_events() {
            if matches!(e, WorldEvent::Dialogue { .. }) {
                w.event_resume();
            }
        }
        if !w.in_event() {
            break;
        }
    }
    assert!(!w.in_event());
    assert_eq!(w.farmer.money, money + 500);
    assert_eq!(w.current, "FarmHouse");
    assert!(w.clock.time >= 2200);
    // Doesn't restart the same day.
    w.warp_farmer("Town", TilePos::new(2, 15), None);
    assert!(!w.in_event());
}

#[test]
fn festival_not_open_before_start() {
    let mut cm = ContentManager::dev_only();
    let mut w = World::new(&mut cm, NewGame::default());
    w.clock.season = Season::Fall;
    w.clock.day = 16;
    w.clock.time = 700;
    w.farmer.events_seen.push("Dev_Intro".into());
    w.drain_events();
    w.warp_farmer("Town", TilePos::new(2, 15), None);
    assert!(!w.in_event());
    assert!(w.drain_events().iter().any(|e| matches!(e, WorldEvent::Message(m) if m.contains("hasn't started"))));
}
