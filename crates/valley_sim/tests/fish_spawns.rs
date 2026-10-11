use valley_content::data::{FishAreaData, LocationData, Rect, SpawnFishData};
use valley_content::ContentManager;
use valley_sim::world::{NewGame, World};
use valley_sim::TilePos;

fn entry(id: &str) -> SpawnFishData {
    SpawnFishData { item_id: Some(format!("(O){id}")), chance: Some(1.0), max_distance_from_shore: -1, catch_limit: -1, can_be_inherited: true, ..Default::default() }
}

fn pick(w: &World, bobber: TilePos) -> Option<String> {
    w.choose_fish_at(&mut engine::Rng::new(3), bobber).map(|f| f.id)
}

#[test]
fn fish_entries_honour_areas_limits_precedence_and_defaults() {
    let mut cm = ContentManager::dev_only();
    let mut w = World::new(&mut cm, NewGame::default());
    w.warp_farmer("Farm", TilePos::new(37, 24), None);
    let pond = TilePos::new(41, 25);
    assert!(w.location().is_water(pond));
    let mut d = LocationData::default();
    d.fish_areas.insert("Deep".into(), FishAreaData { position: Some(Rect { x: 41, y: 24, width: 2, height: 2 }), ..Default::default() });
    let mut legend = entry("Dev_Darter");
    legend.catch_limit = 1;
    legend.fish_area_id = Some("Deep".into());
    legend.precedence = -10;
    legend.set_flag_on_catch = Some("CaughtLegend".into());
    let mut perch = entry("Dev_Perch");
    perch.precedence = 0;
    d.fish = vec![legend, perch];
    // Data/Locations "Farm" for the standard farm is Farm_Standard or Farm.
    for key in ["Farm", "Farm_Standard"] {
        w.location_data.insert(key.into(), d.clone());
    }
    // In the deep area the legendary fish comes first.
    assert_eq!(pick(&w, pond).as_deref(), Some("Dev_Darter"));
    // Outside its area: the perch.
    assert_eq!(pick(&w, TilePos::new(39, 27)).as_deref(), Some("Dev_Perch"));
    // Once caught, never again.
    w.farmer.fish_caught.insert("Dev_Darter".into(), (1, 10));
    assert_eq!(pick(&w, pond).as_deref(), Some("Dev_Perch"));
    // Nothing local: the inheritable Default entries apply.
    for key in ["Farm", "Farm_Standard"] {
        w.location_data.insert(key.into(), LocationData::default());
    }
    let mut def = LocationData::default();
    def.fish = vec![entry("Dev_Crawler")];
    w.location_data.insert("Default".into(), def);
    assert_eq!(pick(&w, pond).as_deref(), Some("Dev_Crawler"));
    // Distance from shore.
    assert_eq!(w.distance_from_shore(pond), 3);
}
