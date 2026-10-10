use valley_content::ContentManager;
use valley_sim::item::{Item, Tool, ToolKind};
use valley_sim::save::SaveData;
use valley_sim::time::Weather;
use valley_sim::world::{NewGame, World};
use valley_sim::{Dir, TilePos};

fn world() -> (ContentManager, World) {
    let mut cm = ContentManager::dev_only();
    let w = World::new(&mut cm, NewGame::default());
    (cm, w)
}

/// Put the pet right below the farmer, who faces it.
fn pet_below_farmer(w: &mut World) -> TilePos {
    let me = TilePos::new(14, 12);
    w.warp_farmer("Farm", me, Some(Dir::Down));
    w.farmer.set_tile(me);
    w.farmer.facing = Dir::Down;
    let t = TilePos::new(14, 13);
    let p = w.pet.as_mut().unwrap();
    p.location = "Farm".into();
    p.pos = engine::Vec2::new(t.x as f32 * 64.0, t.y as f32 * 64.0 - 16.0);
    p.target = None;
    t
}

#[test]
fn new_game_has_a_pet_that_can_be_petted_once_a_day() {
    let (_, mut w) = world();
    let pet = w.pet.as_ref().expect("a pet is adopted on a new game");
    assert_eq!(pet.location, "Farm");
    assert_eq!(pet.texture, "Animals/Dev_Dog");
    let t = pet_below_farmer(&mut w);
    assert!(w.do_action(Some(t)));
    assert_eq!(w.pet.as_ref().unwrap().friendship, 12);
    assert!(w.do_action(Some(t)));
    assert_eq!(w.pet.as_ref().unwrap().friendship, 12, "only once per day");
    w.weather_tomorrow = Weather::Sun;
    w.end_day(false);
    assert!(!w.pet.as_ref().unwrap().petted_today);
}

#[test]
fn watering_the_bowl_raises_friendship_next_morning() {
    let (_, mut w) = world();
    let bowl = w.locations["Farm"].map_prop_tile("PetBowlTile").expect("dev farm has a bowl");
    w.warp_farmer("Farm", TilePos::new(bowl.x, bowl.y - 1), Some(Dir::Down));
    w.farmer.inventory.slots[11] = Some(Item::Tool(Tool::new(ToolKind::WateringCan)));
    w.farmer.current_slot = 11;
    w.apply_tool(ToolKind::WateringCan, 11, bowl);
    assert!(w.pet.as_ref().unwrap().bowl_filled);
    w.weather_tomorrow = Weather::Sun;
    w.end_day(false);
    let p = w.pet.as_ref().unwrap();
    assert_eq!(p.friendship, 6);
    assert!(!p.bowl_filled, "the bowl is empty again on a dry morning");
}

#[test]
fn rain_keeps_the_pet_indoors() {
    let (_, mut w) = world();
    w.weather_tomorrow = Weather::Rain;
    w.end_day(false);
    let p = w.pet.as_ref().unwrap();
    assert_eq!(p.location, "FarmHouse");
    assert!(p.bowl_filled, "rain fills the bowl");
    w.weather_tomorrow = Weather::Sun;
    w.end_day(false);
    assert_eq!(w.pet.as_ref().unwrap().location, "Farm");
}

#[test]
fn pet_wanders_without_leaving_open_ground() {
    let (_, mut w) = world();
    let start = w.pet.as_ref().unwrap().pos;
    for _ in 0..600 {
        w.update(&valley_sim::input::PlayerInput::default());
    }
    let p = w.pet.as_ref().unwrap();
    assert_ne!(p.pos, start, "the pet moves around");
    let bb = valley_sim::farm::FarmAnimal::bbox_at(p.pos);
    assert!(!valley_sim::collision::box_blocked(&w.locations["Farm"], bb));
}

#[test]
fn trash_cans_are_searched_once_per_day() {
    let (_, mut w) = world();
    let can = TilePos::new(19, 9);
    let items = |w: &World| w.farmer.inventory.slots.iter().flatten().map(|i| i.stack()).sum::<i32>();
    let mut found = 0;
    for _ in 0..12 {
        w.warp_farmer("Town", TilePos::new(19, 10), Some(Dir::Up));
        let before = items(&w);
        assert!(w.do_action(Some(can)));
        let after_first = items(&w);
        found += (after_first - before).max(0);
        assert!(w.do_action(Some(can)));
        assert_eq!(items(&w), after_first, "a second search finds nothing");
        w.weather_tomorrow = Weather::Sun;
        w.end_day(false);
    }
    assert!(found > 0, "something turns up over twelve days");
}

#[test]
fn achievements_are_awarded_overnight() {
    let (_, mut w) = world();
    assert!(w.farmer.achievements.is_empty());
    w.farmer.total_earned = 20_000;
    w.monsters_killed = 100;
    w.end_day(false);
    assert!(w.farmer.achievements.iter().any(|a| a == "earn_15k"));
    assert!(w.farmer.achievements.iter().any(|a| a == "slay_100"));
    assert!(!w.farmer.achievements.iter().any(|a| a == "earn_100k"));
    let n = w.farmer.achievements.len();
    w.end_day(false);
    assert_eq!(w.farmer.achievements.len(), n, "awarded once");
}

#[test]
fn pet_and_achievements_survive_save() {
    let (mut cm, mut w) = world();
    w.pet.as_mut().unwrap().friendship = 300;
    w.farmer.achievements.push("fish_10".into());
    let json = SaveData::capture(&w).to_json();
    let r = SaveData::from_json(&json).unwrap().restore(&mut cm);
    assert_eq!(r.pet.as_ref().unwrap().friendship, 300);
    assert_eq!(r.pet.as_ref().unwrap().name, "Biscuit");
    assert_eq!(r.farmer.achievements, vec!["fish_10".to_string()]);
}
