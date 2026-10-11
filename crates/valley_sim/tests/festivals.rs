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
                WorldEvent::Choice { options, .. } if options.iter().any(|(id, _)| id == "festival:main") => host_prompt = true,
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
            if matches!(e, WorldEvent::Dialogue { .. } | WorldEvent::Message(_)) {
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

fn start_festival(key: &str, season: Season, day: i32, activity: Option<&str>) -> World {
    let mut cm = ContentManager::dev_only();
    let mut w = World::new(&mut cm, NewGame::default());
    w.farmer.events_seen.push("Dev_Intro".into());
    let mut info = w.festivals["fall16"].clone();
    info.key = key.to_string();
    if let Some(a) = activity {
        info.data.insert("Port.Activity".into(), a.into());
    }
    info.data.insert("Port.Prize".into(), "(O)Dev_Pendant".into());
    w.festivals.insert(key.to_string(), info);
    w.clock.season = season;
    w.clock.day = day;
    w.clock.time = 1000;
    w.warp_farmer("Town", TilePos::new(2, 15), Some(Dir::Right));
    run_until_roaming(&mut w);
    w
}

fn finish(w: &mut World) -> Vec<String> {
    let mut msgs = Vec::new();
    for _ in 0..4000 {
        w.update(&PlayerInput::default());
        for e in w.drain_events() {
            match e {
                WorldEvent::Message(m) => {
                    msgs.push(m);
                    w.event_resume();
                }
                WorldEvent::Dialogue { .. } => w.event_resume(),
                _ => {}
            }
        }
        if !w.in_event() {
            break;
        }
    }
    assert!(!w.in_event());
    msgs
}

#[test]
fn grange_display_is_judged_for_star_tokens() {
    let mut w = start_festival("fall16", Season::Fall, 16, None);
    for (id, n) in [("Dev_Pumpkin", 1), ("Dev_Turnip", 1), ("Dev_Bean", 1), ("Dev_Juice", 1), ("Dev_Perch", 1), ("Dev_GoldBar", 1), ("Dev_Plum", 1), ("Dev_Egg", 1), ("Dev_Milk", 1)] {
        w.farmer.inventory.add(valley_sim::item::Item::object(id, n));
    }
    w.choose("festival:grange");
    let tile = valley_sim::festival_activities::GRANGE_TILE;
    for _ in 0..9 {
        let slot = (0..w.farmer.inventory.capacity()).find(|&s| w.farmer.inventory.get(s).is_some_and(|i| i.object_id().is_some_and(|id| id != "388" && id != "390"))).unwrap();
        w.chest_transfer(tile, true, slot);
    }
    assert_eq!(w.container(tile).unwrap().slots.iter().flatten().count(), 9);
    w.choose("festival:main");
    let msgs = finish(&mut w);
    assert!(msgs.iter().any(|m| m.contains("star tokens")), "{msgs:?}");
    assert!(w.farmer.star_tokens >= 200, "a full, varied display places well ({})", w.farmer.star_tokens);
    assert_eq!(w.farmer.inventory.count("Dev_Pumpkin"), 1, "items come back after judging");
}

#[test]
fn egg_hunt_rewards_a_quick_collector() {
    let mut w = start_festival("spring13", Season::Spring, 13, None);
    w.choose("festival:main");
    assert!(w.activity.hunting);
    // Grab every egg by stepping onto it.
    let eggs: Vec<TilePos> = w.event.as_ref().unwrap().props.iter().filter_map(|p| match p {
        valley_sim::events::EventProp::Object { tile, .. } => Some(*tile),
        _ => None,
    }).collect();
    assert!(eggs.len() >= 9);
    for t in eggs {
        let a = w.event.as_mut().unwrap().actors.iter_mut().find(|a| a.name == "farmer").unwrap();
        a.pos = engine::Vec2::new(t.x as f32 * 64.0, t.y as f32 * 64.0 - 16.0);
        w.update(&PlayerInput::default());
    }
    assert!(w.activity.eggs_found >= 9);
    let msgs = finish(&mut w);
    assert!(msgs.iter().any(|m| m.contains("You win")), "{msgs:?}");
    assert_eq!(w.farmer.inventory.count("Dev_Pendant"), 1, "the prize");
}

#[test]
fn soup_dance_and_gift_exchange() {
    // Soup: a valuable ingredient pleases everyone.
    let mut w = start_festival("summer11", Season::Summer, 11, None);
    w.farmer.inventory.slots[4] = Some(valley_sim::item::Item::object_q("Dev_Pumpkin", 1, valley_sim::item::Quality::Gold));
    w.farmer.current_slot = 4;
    w.choose("festival:soup");
    assert!(w.activity.soup.is_some());
    let before = w.farmer.friendships.get("Tobin").map(|f| f.points).unwrap_or(0);
    w.choose("festival:main");
    finish(&mut w);
    assert!(w.farmer.friendships["Tobin"].points > before);

    // Dance: needs four hearts.
    let mut w = start_festival("spring24", Season::Spring, 24, None);
    w.choose("festival:dance:Mara");
    assert!(w.activity.partner.is_none(), "turned down at low friendship");
    w.farmer.friendship("Mara").points = 1100;
    w.choose("festival:dance:Mara");
    assert_eq!(w.activity.partner.as_deref(), Some("Mara"));
    let pts = w.farmer.friendships["Mara"].points;
    w.choose("festival:main");
    finish(&mut w);
    assert_eq!(w.farmer.friendships["Mara"].points, pts + 100);
}

#[test]
fn a_secret_friend_is_named_before_the_winter_feast() {
    let mut cm = ContentManager::dev_only();
    let mut w = World::new(&mut cm, NewGame::default());
    let mut info = w.festivals["fall16"].clone();
    info.key = "winter25".into();
    w.festivals.insert("winter25".into(), info);
    w.clock.season = Season::Winter;
    w.clock.day = 17;
    w.end_day(false);
    let friend = w.farmer.secret_friend.clone().expect("assigned on the 18th");
    assert!(w.npcs.iter().any(|n| n.name == friend));
    assert!(w.farmer.mailbox.iter().any(|m| m.starts_with("Port_SecretFriend")));
}
