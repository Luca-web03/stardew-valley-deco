use valley_content::data::{ArtifactSpotDrop, LocationData};
use valley_content::ContentManager;
use valley_sim::item::Item;
use valley_sim::objects::{ObjKind, PlacedObject};
use valley_sim::terrain::{HoeDirt, TerrainFeature};
use valley_sim::world::{NewGame, World};
use valley_sim::{Dir, TilePos};

#[test]
fn crops_grow_only_where_planting_is_allowed() {
    let mut cm = ContentManager::dev_only();
    let mut w = World::new(&mut cm, NewGame::default());
    w.event = None;
    w.warp_farmer("Town", TilePos::new(10, 18), Some(Dir::Down));
    w.event = None;
    let t = TilePos::new(10, 19);
    w.location_mut().state.terrain.insert(t, TerrainFeature::HoeDirt(HoeDirt::new()));
    w.farmer.inventory.slots[8] = Some(Item::object("Dev_TurnipSeeds", 3));
    w.farmer.current_slot = 8;
    assert!(!w.can_plant_here());
    w.plant(t, "Dev_TurnipSeeds");
    assert!(matches!(w.location().terrain(t), Some(TerrainFeature::HoeDirt(d)) if d.crop.is_none()));
    // A location can opt in.
    let mut d = w.location_data.get("Town").cloned().unwrap_or_default();
    d.can_plant_here = Some(true);
    w.location_data.insert("Town".into(), d);
    assert!(w.plant(t, "Dev_TurnipSeeds"));
    assert!(matches!(w.location().terrain(t), Some(TerrainFeature::HoeDirt(d)) if d.crop.is_some()));
}

#[test]
fn artifact_spots_use_default_drops_by_precedence() {
    let mut cm = ContentManager::dev_only();
    let mut w = World::new(&mut cm, NewGame::default());
    w.warp_farmer("Farm", TilePos::new(20, 20), Some(Dir::Down));
    let t = TilePos::new(20, 21);
    let drop = |id: &str, precedence: i32, more: bool| ArtifactSpotDrop { item_id: format!("(O){id}"), chance: Some(1.0), min_stack: 1, max_stack: 1, precedence, continue_on_drop: more, apply_generous_enchantment: true, ..Default::default() };
    let mut def = LocationData::default();
    def.artifact_spots = vec![drop("Dev_OldCoin", 5, false), drop("Dev_Quartz", -5, true)];
    w.location_data.insert("Default".into(), def);
    for key in ["Farm", "Farm_Standard"] {
        if let Some(d) = w.location_data.get_mut(key) {
            d.artifact_spots.clear();
        }
    }
    w.location_mut().state.objects.insert(t, PlacedObject::new("Dev_ArtifactSpot", false, ObjKind::ArtifactSpot));
    w.dropped.clear();
    assert!(w.dig_artifact_spot(t));
    let ids: Vec<&str> = w.dropped.iter().filter_map(|d| d.item.object_id()).collect();
    assert_eq!(ids, vec!["Dev_Quartz", "Dev_OldCoin"], "the earlier drop continues on to the next");
}
