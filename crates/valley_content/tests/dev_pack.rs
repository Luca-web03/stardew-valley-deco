use valley_content::data::{CharacterData, CropData, ObjectData, ShopData};
use valley_content::ContentManager;

#[test]
fn placeholder_pack_loads_through_real_loaders() {
    let mut cm = ContentManager::dev_only();
    let farm = cm.load_map("Maps/Farm").expect("farm map");
    assert_eq!(farm.width, 48);
    assert!(farm.prop("Warp").unwrap().contains("Town"));
    assert_eq!(farm.tile_prop("Back", 10, 20, "Diggable").as_deref(), Some("T"));
    assert!(farm.tile("Back", 40, 24).unwrap().anim.is_some());
    assert_eq!(farm.sheets[0].image, "spring_dev_outdoors");

    for m in ["Maps/FarmHouse", "Maps/Town", "Maps/SeedShop"] {
        assert!(cm.load_map(m).is_some(), "{m}");
    }
    assert!(cm.load_image("Maps/spring_dev_outdoors").is_some());
    assert!(cm.load_image("Maps\\summer_dev_outdoors").is_some());
    assert!(cm.load_image("Characters/Mara").is_some());

    let objects = cm.load_dict::<ObjectData>("Data/Objects");
    let turnip = &objects["Dev_Turnip"];
    assert_eq!(cm.expand(&turnip.display_name), "Turnip");
    let crops = cm.load_dict::<CropData>("Data/Crops");
    assert_eq!(crops["Dev_BeanSeeds"].regrow_days, 3);
    assert!(crops["Dev_BeanSeeds"].needs_watering);
    let chars = cm.load_dict::<CharacterData>("Data/Characters");
    assert_eq!(chars["Mara"].home[0].location, "Town");
    let shops = cm.load_dict::<ShopData>("Data/Shops");
    assert_eq!(shops["SeedShop"].items.len(), 11);
    assert_eq!(cm.load_string_dict("Characters/schedules/Mara").len(), 2);
    assert!(cm.warnings.is_empty(), "{:?}", cm.warnings);
}
