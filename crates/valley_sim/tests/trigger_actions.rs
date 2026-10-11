use valley_content::{ContentManager, LayeredSource, MemSource};
use valley_sim::world::{NewGame, World};
use valley_sim::TilePos;

fn with(files: &[(&str, &str)]) -> ContentManager {
    let mut extra = MemSource::new("test");
    for (p, s) in files {
        extra.insert_str(p, s);
    }
    let mut l = LayeredSource::default();
    l.push(Box::new(extra));
    l.push(Box::new(valley_content::dev::build()));
    ContentManager::new(Box::new(l))
}

#[test]
fn trigger_actions_run_once_when_their_condition_passes() {
    let mut cm = with(&[(
        "Data/TriggerActions.json",
        r#"[
            { "Id": "Gift", "Trigger": "DayStarted", "Condition": "DAYS_PLAYED 2",
              "Actions": ["AddMoney 500", "AddMail Current Dev_Note now", "AddItem (O)388 10"] },
            { "Id": "Visit", "Trigger": "LocationChanged", "Condition": "PLAYER_LOCATION_NAME Current Town",
              "Action": "If PLAYER_CURRENT_MONEY Current 100000 ## AddMoney 1 ## AddConversationTopic VisitedTown 3" },
            { "Id": "Every", "Trigger": "DayEnding", "Action": "IncrementStat Port.Nights", "MarkActionApplied": false }
        ]"#,
    )]);
    let mut w = World::new(&mut cm, NewGame::default());
    let money = w.farmer.money;
    w.end_day(false);
    assert_eq!(w.clock.days_played(), 2);
    assert_eq!(w.farmer.money, money + 500);
    assert!(w.farmer.mailbox.contains(&"Dev_Note".to_string()));
    assert!(w.farmer.inventory.count("388") >= 10);
    w.end_day(false);
    assert_eq!(w.farmer.money, money + 500, "applied once");
    assert_eq!(w.farmer.counters.get("Port.Nights"), Some(&2), "repeatable entries run each night");
    w.warp_farmer("Town", TilePos::new(10, 15), None);
    assert!(w.farmer.conversation_topics.contains_key("VisitedTown"), "the If's else branch");
    assert!(w.farmer.trigger_actions_run.contains(&"Visit".to_string()));
}

#[test]
fn queries_see_the_player_and_the_world() {
    let mut cm = ContentManager::dev_only();
    let mut w = World::new(&mut cm, NewGame::default());
    w.farmer.money = 1234;
    w.farmer.friendship("Mara").points = 760;
    w.farmer.inventory.add(valley_sim::item::Item::object("388", 7));
    let mut rng = engine::Rng::new(1);
    let mut ok = |q: &str| w.check_query(Some(q), None, None, &mut rng);
    assert!(ok("PLAYER_CURRENT_MONEY Current 1000 2000"));
    assert!(!ok("PLAYER_CURRENT_MONEY Current 2000"));
    assert!(ok("PLAYER_HEARTS Current Mara 3"));
    assert!(!ok("PLAYER_HEARTS Current Mara 4"));
    assert!(ok("PLAYER_HAS_ITEM Current (O)388 5"));
    assert!(ok("FARM_TYPE Standard"));
    assert!(ok(r#"ANY "SEASON Winter" "PLAYER_GENDER Current Male""#));
    assert!(ok("SYNCED_RANDOM day test 1"));
    assert!(!ok("SYNCED_RANDOM day test 0"));
    let item = valley_sim::item::Item::object("388", 3);
    assert!(w.check_query(Some("ITEM_ID Target (O)388, ITEM_STACK Target 2"), Some(&item), None, &mut engine::Rng::new(1)));
    assert!(!w.check_query(Some("ITEM_ID Input (O)388"), Some(&item), None, &mut engine::Rng::new(1)), "no input item");
}

#[test]
fn mail_and_dialogue_can_run_actions() {
    let mut cm = with(&[("Data/mail.json", r#"{ "Dev_Act": "Here you go!%action AddMoney 42 %%[#]Coins" }"#)]);
    let mut w = World::new(&mut cm, NewGame::default());
    let money = w.farmer.money;
    w.farmer.mailbox.clear();
    w.farmer.mailbox.push("Dev_Act".into());
    w.read_next_mail();
    assert_eq!(w.farmer.money, money + 42);
    let (text, _) = w.prepare_dialogue("Hello!#$action AddMoney 8#$query SEASON Spring#Spring!|Not spring.");
    assert_eq!(w.farmer.money, money + 50);
    assert!(text.contains("Spring!"), "{text}");
}
