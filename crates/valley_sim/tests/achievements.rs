use std::collections::BTreeMap;

use valley_content::ContentManager;
use valley_sim::world::{NewGame, World};

fn world() -> World {
    let mut cm = ContentManager::dev_only();
    World::new(&mut cm, NewGame::default())
}

#[test]
fn engine_list_without_content_data() {
    let mut w = world();
    assert!(w.achievement_data.is_empty());
    w.farmer.total_earned = 20_000;
    w.end_day(false);
    let list = w.achievement_list();
    assert!(list.iter().any(|(id, _, _, got)| id == "earn_15k" && *got));
}

#[test]
fn original_ids_and_names_when_the_data_is_present() {
    let mut w = world();
    let raw: BTreeMap<String, String> = [
        ("0", "Starter Funds^Earn 15,000g^true^-1^-1"),
        ("6", "New Pal^Reach 5 hearts with someone.^true^-1^-1"),
        ("15", "Bigger Place^Upgrade your house.^true^-1^-1"),
        ("99", "Hidden^Something secret.^false^-1^-1"),
    ]
    .iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();
    w.achievement_data = valley_sim::achievements::parse(&raw);
    w.farmer.total_earned = 20_000;
    w.farmer.friendship("Mara").points = 1300;
    w.end_day(false);
    let list = w.achievement_list();
    assert_eq!(list.len(), 3, "hidden ones stay hidden until earned");
    let got: Vec<&str> = list.iter().filter(|a| a.3).map(|a| a.1.as_str()).collect();
    assert_eq!(got, vec!["Starter Funds", "New Pal"]);
    assert!(!w.farmer.achievements.iter().any(|a| a == "earn_15k"), "engine ids aren't used alongside");
}
