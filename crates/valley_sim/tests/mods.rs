use std::path::PathBuf;

use valley_content::ContentManager;
use valley_sim::world::{NewGame, World};

fn temp_mods(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("valley_mods_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    root
}

fn write(path: PathBuf, body: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, body).unwrap();
}

#[test]
fn mods_add_change_and_remove_data_entries() {
    let root = temp_mods("data");
    write(root.join("BetterTurnips/manifest.json"), r#"{ "Name": "Better Turnips", "Version": "1.0" }"#);
    write(
        root.join("BetterTurnips/Data/Objects.json"),
        r#"{
            "Dev_Turnip": { "Name": "Turnip", "DisplayName": "Golden Turnip", "Type": "Basic", "Category": -75, "Price": 999, "SpriteIndex": 1 },
            "Mod_Radish": { "Name": "Radish", "DisplayName": "Radish", "Type": "Basic", "Category": -75, "Price": 40, "SpriteIndex": 1 },
            "Dev_Driftwood": null
        }"#,
    );
    // A second mod loads later and wins on the same key.
    write(root.join("ZRadishFix/Data/Objects.json"), r#"{ "Mod_Radish": { "Name": "Radish", "DisplayName": "Crisp Radish", "Type": "Basic", "Category": -75, "Price": 55, "SpriteIndex": 1 } }"#);
    // Array assets merge by Id.
    write(root.join("ZRadishFix/Data/FishPondData.json"), r#"[{ "Id": "Mod_Pond", "RequiredTags": ["id_o_dev_darter"] }]"#);

    let mut cm = ContentManager::with_mods(None, Some(&root));
    let names: Vec<String> = cm.mods().iter().map(|m| m.name.clone()).collect();
    assert_eq!(names, vec!["Better Turnips", "ZRadishFix"]);
    let w = World::new(&mut cm, NewGame::default());
    assert_eq!(w.items.object("Dev_Turnip").unwrap().data.price, 999);
    assert_eq!(w.items.object("Dev_Turnip").unwrap().display_name, "Golden Turnip");
    assert_eq!(w.items.object("Mod_Radish").unwrap().data.price, 55);
    assert!(w.items.object("Dev_Driftwood").is_none(), "null removes an entry");
    assert!(w.items.object("Dev_Perch").is_some(), "untouched entries stay");
    let ids: Vec<&str> = w.fish_pond_data.iter().map(|d| d.id.as_str()).collect();
    assert_eq!(ids, vec!["Dev_Perch", "Dev_AnyFish", "Mod_Pond"]);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn mod_files_replace_textures_and_without_mods_nothing_changes() {
    let root = temp_mods("png");
    let mut img = engine::Image::new(2, 2);
    img.set(0, 0, engine::Color::rgb(1, 2, 3));
    std::fs::create_dir_all(root.join("Art/Animals")).unwrap();
    std::fs::write(root.join("Art/Animals/Dev_Dog.png"), img.encode_png()).unwrap();
    let mut cm = ContentManager::with_mods(None, Some(&root));
    let loaded = cm.load_image("Animals/Dev_Dog").unwrap();
    assert_eq!((loaded.width, loaded.height), (2, 2));
    let mut plain = ContentManager::with_mods(None, None);
    assert_eq!(plain.load_image("Animals/Dev_Dog").unwrap().width, 128);
    assert!(plain.mods().is_empty());
    let _ = std::fs::remove_dir_all(&root);
}
