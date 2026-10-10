use valley_content::ContentManager;
use valley_sim::item::Item;
use valley_sim::world::{NewGame, World};

fn world() -> World {
    let mut cm = ContentManager::dev_only();
    World::new(&mut cm, NewGame::default())
}

#[test]
fn completing_bundles_and_an_area() {
    let mut w = world();
    assert_eq!(w.bundle_areas(), vec!["Garden", "Workshop"]);
    assert!(w.donate_to_bundle("Workshop/0", 0).is_err(), "no wood yet");
    w.farmer.inventory.add(Item::object("388", 60));
    w.farmer.inventory.add(Item::object("390", 60));
    w.farmer.inventory.add(Item::object("771", 20));
    for i in 0..3 {
        w.donate_to_bundle("Workshop/0", i).unwrap();
    }
    assert!(w.farmer.bundles.completed.contains(&"Workshop/0".to_string()));
    assert_eq!(w.farmer.inventory.count("388"), 10);
    assert!(w.farmer.inventory.slots.iter().flatten().any(|i| i.big_id() == Some("Dev_Juicer")), "reward received");
    // Money bundle.
    w.farmer.money = 1500;
    w.donate_to_bundle("Workshop/2", 0).unwrap();
    assert_eq!(w.farmer.money, 500);
    // Last bundle completes the area.
    w.farmer.inventory.add(Item::object("Dev_CopperBar", 2));
    w.farmer.inventory.add(Item::object("Dev_Quartz", 1));
    w.farmer.inventory.add(Item::object("Dev_Coal", 5));
    for i in 0..3 {
        w.donate_to_bundle("Workshop/1", i).unwrap();
    }
    assert!(w.farmer.bundles.areas_completed.contains(&"Workshop".to_string()));
    assert!(w.farmer.mail_received.contains(&"ccDone_Workshop".to_string()));
}

#[test]
fn partial_bundles_complete_at_num_required() {
    let mut w = world();
    w.farmer.inventory.add(Item::object("Dev_Perch", 1));
    w.farmer.inventory.add(Item::object("Dev_Darter", 1));
    w.donate_to_bundle("Garden/1", 0).unwrap();
    assert!(!w.farmer.bundles.completed.contains(&"Garden/1".to_string()));
    w.donate_to_bundle("Garden/1", 1).unwrap();
    assert!(w.farmer.bundles.completed.contains(&"Garden/1".to_string()));
    assert!(w.donate_to_bundle("Garden/1", 0).is_err());
}

#[test]
fn museum_takes_minerals_once_and_rewards() {
    let mut w = world();
    w.farmer.inventory.slots[20] = Some(Item::object("Dev_Quartz", 2));
    w.farmer.inventory.slots[21] = Some(Item::object("Dev_Turnip", 1));
    assert!(w.donate_to_museum(21).is_err());
    w.donate_to_museum(20).unwrap();
    assert_eq!(w.farmer.inventory.count("Dev_SpeedGro"), 5, "first-donation reward");
    assert!(w.donate_to_museum(20).is_err(), "already donated");
    w.farmer.inventory.slots[22] = Some(Item::object("Dev_Amberite", 1));
    w.donate_to_museum(22).unwrap();
    assert!(w.farmer.inventory.slots.iter().flatten().any(|i| i.big_id() == Some("Dev_Furnace")));
}
