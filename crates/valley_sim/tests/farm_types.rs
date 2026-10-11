use valley_content::ContentManager;
use valley_sim::objects::ObjKind;
use valley_sim::save::SaveData;
use valley_sim::world::{NewGame, World};
use valley_sim::TilePos;

fn world(kind: &str) -> (ContentManager, World) {
    let mut cm = ContentManager::dev_only();
    let w = World::new(&mut cm, NewGame { farm_type: kind.into(), ..Default::default() });
    (cm, w)
}

#[test]
fn choices_include_the_variants_the_content_has() {
    let mut cm = ContentManager::dev_only();
    let ids: Vec<String> = valley_sim::farm_types::available(&mut cm).into_iter().map(|c| c.id).collect();
    assert_eq!(ids, vec!["Standard", "Riverland", "Hilltop", "Wilderness", "Meadowlands"]);
}

#[test]
fn riverland_has_its_river_and_fish_and_survives_a_reload() {
    let (mut cm, w) = world("Riverland");
    let farm = &w.locations["Farm"];
    assert!(farm.is_water(TilePos::new(3, 20)));
    let fish: Vec<String> = w.location_data_for("Farm").unwrap().fish.iter().map(|f| f.item_id.clone().unwrap_or_default()).collect();
    assert!(fish.iter().any(|f| f.contains("Dev_Darter")), "{fish:?}");
    let save = SaveData::capture(&w).to_json();
    let w2 = SaveData::from_json(&save).unwrap().restore(&mut cm);
    assert!(w2.locations["Farm"].is_water(TilePos::new(3, 20)), "the river is still there after loading");
    let (_, std) = world("Standard");
    assert!(!std.locations["Farm"].is_water(TilePos::new(3, 20)));
}

#[test]
fn hilltop_grows_rocks_in_its_quarry() {
    let (_, mut w) = world("Hilltop");
    let in_quarry = |w: &World| w.locations["Farm"].state.objects.iter().filter(|(p, o)| (1..9).contains(&p.x) && (2..8).contains(&p.y) && matches!(o.kind, ObjKind::MineRock { .. })).count();
    let before = in_quarry(&w);
    w.end_day(false);
    w.end_day(false);
    assert!(in_quarry(&w) > before);
}

#[test]
fn wilderness_brings_monsters_out_at_night() {
    let (_, mut w) = world("Wilderness");
    w.clock.time = 1800;
    for _ in 0..60 * 7 * 30 {
        w.update(&Default::default());
        if w.monsters.iter().any(|m| m.location == "Farm") {
            break;
        }
        w.farmer.stamina = w.farmer.max_stamina as f32;
    }
    assert!(w.monsters.iter().any(|m| m.location == "Farm"));
    let (_, mut std) = world("Standard");
    std.clock.time = 2000;
    for _ in 0..600 {
        std.update(&Default::default());
    }
    assert!(!std.monsters.iter().any(|m| m.location == "Farm"));
}

#[test]
fn meadowlands_starts_with_a_coop_and_two_animals() {
    let (_, w) = world("Meadowlands");
    let coop = w.buildings.iter().find(|b| b.kind == "Coop").expect("coop");
    assert!(coop.interior.is_some());
    assert_eq!(w.animals.len(), 2);
    assert_eq!(w.farmer.money, 500, "the gift is free");
}

#[test]
fn letters_start_quests_and_building_quests_complete() {
    let (_, mut w) = world("Standard");
    w.quest_data.insert("Dev_BuildCoop".into(), "Building/Room to Roost/Build a henhouse./Build a coop./Coop/-1/300//false/".into());
    w.mail_data.insert("Dev_CoopLetter".into(), "@,^Thought you might want some hens.%item quest Dev_BuildCoop %%[#]Hens".into());
    w.farmer.mailbox.clear();
    w.farmer.mailbox.push("Dev_CoopLetter".into());
    w.warp_farmer("Farm", TilePos::new(25, 10), Some(valley_sim::Dir::Up));
    assert!(w.do_action(Some(TilePos::new(25, 9))));
    assert!(w.farmer.quests.iter().any(|q| q.id == "Dev_BuildCoop"));
    let money = w.farmer.money;
    w.grant_building("Coop", TilePos::new(10, 20)).unwrap();
    w.end_day(false);
    assert!(!w.farmer.quests.iter().any(|q| q.id == "Dev_BuildCoop"), "completed");
    assert_eq!(w.farmer.money, money + 300);
}
