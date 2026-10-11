use valley_content::ContentManager;
use valley_sim::bushes::{MEDIUM, TEA};
use valley_sim::item::{Item, Tool, ToolKind};
use valley_sim::terrain::TerrainFeature;
use valley_sim::world::{NewGame, World};
use valley_sim::{Dir, TilePos};

fn world() -> World {
    let mut cm = ContentManager::dev_only();
    World::new(&mut cm, NewGame::default())
}

#[test]
fn paths_layer_places_bushes_that_block_their_width() {
    let w = world();
    let farm = &w.locations["Farm"];
    let at = TilePos::new(2, 12);
    assert!(matches!(farm.terrain(at), Some(TerrainFeature::Bush(b)) if b.size == MEDIUM));
    assert!(!valley_sim::collision::tile_passable(farm, at));
    assert!(!valley_sim::collision::tile_passable(farm, TilePos::new(3, 12)), "a medium bush is two tiles wide");
    assert!(matches!(w.locations["Town"].terrain(TilePos::new(29, 19)), Some(TerrainFeature::Bush(_))));
}

#[test]
fn medium_bushes_give_berries_in_season() {
    let mut w = world();
    w.warp_farmer("Farm", TilePos::new(4, 12), Some(Dir::Left));
    // Not berry season yet.
    w.dropped.clear();
    assert!(w.do_action(Some(TilePos::new(3, 12))));
    assert!(w.dropped.is_empty());
    while w.clock.day < 15 {
        w.end_day(false);
    }
    w.warp_farmer("Farm", TilePos::new(4, 12), Some(Dir::Left));
    w.dropped.clear();
    assert!(w.do_action(Some(TilePos::new(3, 12))), "shaking the right half works too");
    let berries: i32 = w.dropped.iter().filter(|d| d.item.object_id() == Some("Dev_Berry")).map(|d| d.item.stack()).sum();
    assert!(berries >= 1);
    // Once a day.
    w.dropped.clear();
    w.do_action(Some(TilePos::new(3, 12)));
    assert!(w.dropped.is_empty());
}

#[test]
fn tea_bushes_grow_and_leaf_late_in_the_season() {
    let mut w = world();
    w.warp_farmer("Farm", TilePos::new(20, 20), Some(Dir::Down));
    let t = TilePos::new(20, 21);
    w.location_mut().state.objects.remove(&t);
    w.location_mut().state.terrain.remove(&t);
    w.farmer.inventory.slots[8] = Some(Item::object("Dev_TeaSapling", 1));
    w.farmer.current_slot = 8;
    assert!(w.do_action(Some(t)));
    assert!(matches!(w.location().terrain(t), Some(TerrainFeature::Bush(b)) if b.size == TEA));
    // Grown after 20 days, leaves from the 22nd.
    for _ in 0..21 {
        w.end_day(false);
    }
    assert_eq!(w.clock.day, 22);
    w.warp_farmer("Farm", TilePos::new(20, 20), Some(Dir::Down));
    w.dropped.clear();
    assert!(w.do_action(Some(t)));
    assert!(w.dropped.iter().any(|d| d.item.object_id() == Some("Dev_TeaLeaves")));
    // An axe takes it back as a sapling.
    w.farmer.inventory.slots[0] = Some(Item::Tool(Tool::new(ToolKind::Axe)));
    w.dropped.clear();
    w.apply_tool(ToolKind::Axe, 0, t);
    assert!(w.location().terrain(t).is_none());
    assert!(w.dropped.iter().any(|d| d.item.object_id() == Some("Dev_TeaSapling")));
}
