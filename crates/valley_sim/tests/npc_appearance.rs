use valley_content::{ContentManager, LayeredSource, MemSource};
use valley_sim::world::{NewGame, World};

fn cm_with(chars: &str) -> ContentManager {
    let mut extra = MemSource::new("test");
    extra.insert_str("Data/Characters.json", chars);
    let mut l = LayeredSource::default();
    l.push(Box::new(extra));
    l.push(Box::new(valley_content::dev::build()));
    ContentManager::new(Box::new(l))
}

#[test]
fn outfits_follow_season_and_unlocks_hide_npcs() {
    let mut cm = cm_with(
        r#"{
        "Mara": { "DisplayName": "Mara", "Home": [{ "Id": "Default", "Location": "Town", "Tile": { "X": 12, "Y": 18 }, "Direction": "down" }],
                  "Appearance": [
                    { "Id": "Spring", "Season": "Spring", "Outdoors": true, "Indoors": false, "Sprite": "Characters\\Tobin", "Portrait": "Portraits\\Tobin" },
                    { "Id": "Fall", "Season": "Fall", "Sprite": "Characters\\Mara" }
                  ] },
        "Tobin": { "DisplayName": "Tobin", "Home": [{ "Id": "Default", "Location": "Town", "Tile": { "X": 20, "Y": 18 }, "Direction": "down" }],
                   "UnlockConditions": "YEAR 2" }
    }"#,
    );
    let mut w = World::new(&mut cm, NewGame::default());
    w.start_day();
    let mara = w.npcs.iter().find(|n| n.name == "Mara").unwrap();
    assert_eq!(mara.look_outdoors.0, "Characters/Tobin", "spring outdoor outfit");
    assert_eq!(mara.look_indoors.0, "Characters/Mara", "no indoor match: the default");
    assert_eq!(mara.texture, "Characters/Tobin", "she's outdoors in town");
    let tobin = w.npcs.iter().find(|n| n.name == "Tobin").unwrap();
    assert!(tobin.location.is_empty(), "not unlocked in year 1");
}
