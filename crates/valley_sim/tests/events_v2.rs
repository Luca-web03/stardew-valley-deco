use valley_content::ContentManager;
use valley_sim::events::EventProp;
use valley_sim::input::PlayerInput;
use valley_sim::world::{NewGame, World, WorldEvent};
use valley_sim::{Dir, TilePos};

fn world() -> World {
    let mut cm = ContentManager::dev_only();
    let mut w = World::new(&mut cm, NewGame::default());
    w.farmer.events_seen.push("Dev_Intro".into());
    w
}

/// Run the current event to the end, answering questions with `answer`.
/// Returns the dialogue texts shown.
fn play(w: &mut World, answer: usize, mut each_tick: impl FnMut(&World)) -> Vec<String> {
    let mut said = Vec::new();
    for _ in 0..60 * 120 {
        w.update(&PlayerInput::default());
        each_tick(w);
        for e in w.drain_events() {
            match e {
                WorldEvent::Dialogue { pages, .. } => {
                    said.extend(pages.into_iter().map(|p| p.text));
                    w.event_resume();
                }
                WorldEvent::Message(m) => {
                    said.push(m);
                    w.event_resume();
                }
                WorldEvent::EventQuestion { options, .. } => {
                    assert!(options.len() > answer);
                    // The UI closes the question menu (resume) before the answer lands.
                    w.event_resume();
                    w.event_answer(answer);
                }
                _ => {}
            }
        }
        if !w.in_event() {
            break;
        }
    }
    assert!(!w.in_event(), "event finished");
    said
}

fn add_events(w: &mut World, entries: &[(&str, &str)]) {
    let town = w.event_data.entry("Town".into()).or_default();
    for (k, v) in entries {
        town.insert(k.to_string(), v.to_string());
    }
}

#[test]
fn question_forks_into_another_script() {
    for (answer, expect) in [(0, "Glad you came"), (1, "Maybe next time")] {
        let mut w = world();
        add_events(&mut w, &[
            ("900/t 600 2600", "none/10 10/farmer 10 12 0 Mara 10 10 2/question fork1 \"Will you help?#Of course#Not now\"/fork leave/speak Mara \"Glad you came!\"/end"),
            ("leave", "speak Mara \"Maybe next time.\"/end warpOut"),
        ]);
        w.warp_farmer("Town", TilePos::new(10, 12), Some(Dir::Up));
        assert!(w.in_event());
        let said = play(&mut w, answer, |_| {});
        assert!(said.iter().any(|s| s.contains(expect)), "{said:?}");
        assert!(w.farmer.events_seen.contains(&"900".to_string()));
    }
}

#[test]
fn quick_questions_run_their_branch() {
    let mut w = world();
    add_events(&mut w, &[("901/t 600 2600", "none/10 10/farmer 10 12 0 Mara 10 10 2/quickQuestion Tea or coffee?#Tea#Coffee(break)speak Mara \"Tea it is.\"\\friendship Mara 30(break)speak Mara \"Coffee, then.\"/speak Mara \"Enjoy!\"/end")]);
    w.warp_farmer("Town", TilePos::new(10, 12), Some(Dir::Up));
    let said = play(&mut w, 0, |_| {});
    let i_tea = said.iter().position(|s| s.contains("Tea it is")).expect("branch ran");
    let i_enjoy = said.iter().position(|s| s.contains("Enjoy")).expect("script continues");
    assert!(i_tea < i_enjoy);
    assert!(!said.iter().any(|s| s.contains("Coffee, then")));
    assert_eq!(w.farmer.friendships["Mara"].points, 30);
}

#[test]
fn actors_animate_hop_and_talk_above_their_heads() {
    let mut w = world();
    add_events(&mut w, &[(
        "902/t 600 2600",
        "none/10 10/farmer 10 12 0 Mara 10 10 2/showFrame Mara 9/pause 100/animate Mara false true 100 12 13 14/jump Mara 6/textAboveHead Mara \"Wheee!\"/pause 300/stopAnimation Mara 3/pause 50/end",
    )]);
    w.warp_farmer("Town", TilePos::new(10, 12), Some(Dir::Up));
    let mut saw_frame9 = false;
    let mut saw_anim = false;
    let mut saw_hop = false;
    let mut saw_speech = false;
    let mut end_frame = None;
    play(&mut w, 0, |w| {
        if let Some(e) = &w.event {
            let m = e.actors.iter().find(|a| a.name == "Mara").unwrap();
            saw_frame9 |= m.sprite_frame() == Some(9);
            saw_anim |= matches!(m.sprite_frame(), Some(12..=14));
            saw_hop |= m.z > 0.0;
            saw_speech |= m.speech.as_ref().is_some_and(|s| s.0 == "Wheee!");
            end_frame = m.sprite_frame();
        }
    });
    assert!(saw_frame9 && saw_anim && saw_hop && saw_speech);
    assert_eq!(end_frame, Some(3));
}

#[test]
fn props_map_edits_and_rewards_last_only_while_relevant() {
    let mut w = world();
    let before = w.locations["Town"].map.layer("Buildings").and_then(|l| l.get(12, 12)).map(|t| t.index);
    add_events(&mut w, &[(
        "903/t 600 2600",
        "none/10 10/farmer 10 12 0 Mara 10 10 2/addObject 11 11 Dev_Turnip/changeMapTile Buildings 12 12 26/temporaryAnimatedSprite Maps\\\\springobjects 0 0 16 16 100 2 3 9 9 false false 0.9 0 4 0 0 0/pause 200/addCookingRecipe Garden Salad/addItem (O)Dev_Bean 3/eventSeen 555/end dialogue Mara \"About earlier... thanks.\"",
    )]);
    w.warp_farmer("Town", TilePos::new(10, 12), Some(Dir::Up));
    let mut saw_prop = false;
    let mut saw_tile = false;
    play(&mut w, 0, |w| {
        if let Some(e) = &w.event {
            saw_prop |= e.props.iter().any(|p| matches!(p, EventProp::Object { id, .. } if id == "Dev_Turnip"));
            saw_tile |= w.locations["Town"].map.layer("Buildings").and_then(|l| l.get(12, 12)).map(|t| t.index) == Some(26);
        }
    });
    assert!(saw_prop && saw_tile);
    let after = w.locations["Town"].map.layer("Buildings").and_then(|l| l.get(12, 12)).map(|t| t.index);
    assert_eq!(before, after, "map restored after the event");
    assert!(w.farmer.cooking_recipes.contains_key("Garden Salad"));
    assert_eq!(w.farmer.inventory.count("Dev_Bean"), 3);
    assert!(w.farmer.events_seen.contains(&"555".to_string()));
    // The next talk with Mara uses the event's line once.
    let i = w.npcs.iter().position(|n| n.name == "Mara").unwrap();
    let t = w.npcs[i].tile();
    w.warp_farmer(&w.npcs[i].location.clone(), TilePos::new(t.x, t.y + 1), Some(Dir::Up));
    w.npcs[i].set_tile(t);
    w.drain_events();
    w.do_action(Some(t));
    let said = w.drain_events().into_iter().find_map(|e| match e {
        WorldEvent::Dialogue { pages, .. } => Some(pages[0].text.clone()),
        _ => None,
    });
    assert!(said.unwrap_or_default().contains("About earlier"));
}

#[test]
fn end_variants_place_the_farmer() {
    let mut w = world();
    add_events(&mut w, &[("904/t 600 2600", "none/10 10/farmer 10 12 0/move farmer 0 -2 0/end position 5 6")]);
    w.warp_farmer("Town", TilePos::new(10, 12), Some(Dir::Up));
    play(&mut w, 0, |_| {});
    assert_eq!(w.farmer.tile(), TilePos::new(5, 6));

    let mut w = world();
    add_events(&mut w, &[("905/t 600 2600", "none/10 10/farmer 10 12 0/move farmer 0 -2 0/end")]);
    w.warp_farmer("Town", TilePos::new(10, 12), Some(Dir::Up));
    play(&mut w, 0, |_| {});
    assert_eq!(w.farmer.tile(), TilePos::new(10, 12), "plain end returns the farmer where they were");
}
