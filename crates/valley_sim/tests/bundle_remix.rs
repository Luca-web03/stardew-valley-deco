use valley_content::ContentManager;
use valley_sim::save::SaveData;
use valley_sim::world::{NewGame, World};

fn remixed(seed: u64) -> World {
    let mut cm = ContentManager::dev_only();
    World::new(&mut cm, NewGame { remixed_bundles: true, seed, ..Default::default() })
}

#[test]
fn remixed_bundles_fill_every_slot_with_known_items() {
    let w = remixed(7);
    let keys: Vec<&str> = w.bundles.iter().map(|b| b.key.as_str()).collect();
    assert_eq!(keys, ["Garden/0", "Garden/1", "Workshop/0", "Workshop/1", "Workshop/2"]);
    for b in &w.bundles {
        assert!(!b.reqs.is_empty(), "{} has items", b.name);
        assert!(b.num_required <= b.reqs.len());
        for r in &b.reqs {
            assert!(r.item_id == "-1" || w.items.objects.contains_key(&r.item_id), "{} resolves", r.item_id);
        }
    }
    let fund = w.bundles.iter().find(|b| b.key == "Workshop/2").unwrap();
    assert!(fund.is_money());
    assert_eq!(fund.name, "Repair Fund");
    // Picks trim the item lists.
    let builders = w.bundles.iter().find(|b| b.key == "Workshop/0").unwrap();
    assert_eq!(builders.reqs.len(), 3);
    assert!(builders.reward.is_some());
}

#[test]
fn remixes_differ_by_seed_and_survive_saving() {
    let names = |w: &World| w.bundles.iter().map(|b| format!("{}:{:?}", b.name, b.reqs.iter().map(|r| r.item_id.clone()).collect::<Vec<_>>())).collect::<Vec<_>>();
    let all: std::collections::BTreeSet<Vec<String>> = (1..12).map(|s| names(&remixed(s))).collect();
    assert!(all.len() > 1, "different seeds roll different bundles");

    let w = remixed(3);
    let json = SaveData::capture(&w).to_json();
    let mut cm = ContentManager::dev_only();
    let back = SaveData::from_json(&json).unwrap().restore(&mut cm);
    assert_eq!(names(&back), names(&w));

    // Standard games keep Data/Bundles.
    let mut cm = ContentManager::dev_only();
    let std = World::new(&mut cm, NewGame::default());
    assert_eq!(std.bundles.iter().find(|b| b.key == "Garden/0").unwrap().name, "Spring Basket");
    assert!(std.farmer.bundle_data.is_empty());
}
