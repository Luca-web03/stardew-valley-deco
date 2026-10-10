use valley_content::ContentManager;
use valley_sim::item::{Item, ToolKind};
use valley_sim::objects::{ObjKind, PlacedObject};
use valley_sim::terrain::{HoeDirt, TerrainFeature, Tree};
use valley_sim::time::Season;
use valley_sim::world::{NewGame, World};
use valley_sim::{Dir, TilePos};

fn world_at_farm() -> World {
    let mut cm = ContentManager::dev_only();
    let mut w = World::new(&mut cm, NewGame::default());
    w.warp_farmer("Farm", TilePos::new(12, 18), Some(Dir::Down));
    w.farmer.set_tile(TilePos::new(12, 18));
    w.farmer.facing = Dir::Down;
    let loc = w.location_mut();
    for y in 17..23 {
        for x in 10..16 {
            loc.state.objects.remove(&TilePos::new(x, y));
            loc.state.terrain.remove(&TilePos::new(x, y));
        }
    }
    w
}

#[test]
fn fruit_tree_matures_and_fruits_in_season() {
    let mut w = world_at_farm();
    w.farmer.inventory.slots[11] = Some(Item::object("Dev_PlumSapling", 1));
    w.farmer.current_slot = 11;
    assert!(w.do_action(None));
    let t = TilePos::new(12, 19);
    assert!(matches!(w.location().terrain(t), Some(TerrainFeature::FruitTree(_))));
    w.clock.season = Season::Summer;
    w.clock.day = 1;
    for _ in 0..30 {
        w.end_day(false);
        w.clock.season = Season::Summer;
    }
    let Some(TerrainFeature::FruitTree(ft)) = w.locations["Farm"].terrain(t) else { panic!() };
    assert!(ft.mature());
    assert!(!ft.fruit.is_empty() && ft.fruit.len() <= 3);
    w.warp_farmer("Farm", TilePos::new(12, 18), Some(Dir::Down));
    w.farmer.set_tile(TilePos::new(12, 18));
    w.farmer.facing = Dir::Down;
    w.farmer.current_slot = 30;
    assert!(w.do_action(None));
    for _ in 0..120 {
        w.update(&Default::default());
    }
    let ground: i32 = w.dropped.iter().filter(|d| d.item.object_id() == Some("Dev_Plum")).map(|d| d.item.stack()).sum();
    assert!(w.farmer.inventory.count("Dev_Plum") + ground >= 1);
}

#[test]
fn tapper_makes_syrup() {
    let mut w = world_at_farm();
    let t = TilePos::new(12, 19);
    w.location_mut().state.terrain.insert(t, TerrainFeature::Tree(Tree::new("1", 5)));
    w.farmer.inventory.slots[11] = Some(Item::big("Dev_Tapper", 1));
    w.farmer.current_slot = 11;
    assert!(w.do_action(None));
    assert!(matches!(w.location().object(t).unwrap().kind, ObjKind::Tapper { .. }));
    for _ in 0..5 {
        w.end_day(false);
    }
    w.warp_farmer("Farm", TilePos::new(12, 18), Some(Dir::Down));
    w.farmer.set_tile(TilePos::new(12, 18));
    w.farmer.facing = Dir::Down;
    w.farmer.current_slot = 30;
    assert!(w.do_action(None));
    assert_eq!(w.farmer.inventory.count("Dev_Syrup"), 1);
}

#[test]
fn artifact_spot_digs_up_something() {
    let mut w = world_at_farm();
    w.warp_farmer("Town", TilePos::new(5, 18), Some(Dir::Down));
    w.event = None;
    let t = TilePos::new(5, 19);
    w.location_mut().state.objects.insert(t, PlacedObject::new("Dev_ArtifactSpot", false, ObjKind::ArtifactSpot));
    w.apply_tool(ToolKind::Hoe, 1, t);
    assert!(w.location().object(t).is_none());
    assert!(!w.dropped.is_empty());
}

#[test]
fn greenhouse_grows_out_of_season() {
    let mut w = world_at_farm();
    w.warp_farmer("Greenhouse", TilePos::new(4, 6), Some(Dir::Up));
    assert!(!w.location().outdoors);
    w.clock.season = Season::Winter;
    let t = TilePos::new(4, 5);
    w.location_mut().state.terrain.insert(t, TerrainFeature::HoeDirt(HoeDirt::new()));
    w.farmer.set_tile(TilePos::new(4, 6));
    w.farmer.facing = Dir::Up;
    w.farmer.inventory.slots[11] = Some(Item::object("Dev_TurnipSeeds", 1));
    w.farmer.current_slot = 11;
    assert!(w.do_action(None));
    for _ in 0..4 {
        if let Some(TerrainFeature::HoeDirt(d)) = w.locations.get_mut("Greenhouse").unwrap().state.terrain.get_mut(&t) {
            d.watered = true;
        }
        w.end_day(false);
    }
    let Some(TerrainFeature::HoeDirt(d)) = w.locations["Greenhouse"].terrain(t) else { panic!() };
    let c = d.crop.as_ref().unwrap();
    assert!(!c.dead);
    assert!(c.is_ready(&w.items.crops["Dev_TurnipSeeds"]));
}
