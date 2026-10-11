use valley_content::ContentManager;
use valley_sim::item::{Item, Tool, ToolKind};
use valley_sim::objects::ObjKind;
use valley_sim::terrain::TerrainFeature;
use valley_sim::world::{NewGame, World};
use valley_sim::{Dir, TilePos};

fn world() -> World {
    let mut cm = ContentManager::dev_only();
    let mut w = World::new(&mut cm, NewGame::default());
    let farm = w.locations.get_mut("Farm").unwrap();
    for y in 14..32 {
        for x in 6..36 {
            farm.state.objects.remove(&TilePos::new(x, y));
            farm.state.terrain.remove(&TilePos::new(x, y));
        }
    }
    farm.state.clumps.clear();
    w.warp_farmer("Farm", TilePos::new(20, 20), Some(Dir::Down));
    w
}

fn hold(w: &mut World, item: Item) {
    w.farmer.inventory.slots[8] = Some(item);
    w.farmer.current_slot = 8;
}

#[test]
fn fences_block_wear_down_and_come_back_with_the_right_tool() {
    let mut w = world();
    hold(&mut w, Item::object("Dev_WoodFence", 3));
    let t = TilePos::new(20, 21);
    assert!(w.place_current(t));
    assert!(matches!(w.location().object(t).unwrap().kind, ObjKind::Fence { health, .. } if health >= 28));
    assert!(!valley_sim::collision::tile_passable(w.location(), t), "fences block");
    // A pickaxe doesn't take a wood fence; an axe does.
    w.apply_tool(ToolKind::Pickaxe, 0, t);
    assert!(w.location().object(t).is_some());
    w.farmer.inventory.slots[0] = Some(Item::Tool(Tool::new(ToolKind::Axe)));
    w.dropped.clear();
    w.apply_tool(ToolKind::Axe, 0, t);
    assert!(w.location().object(t).is_none());
    assert!(w.dropped.iter().any(|d| d.item.object_id() == Some("Dev_WoodFence")));

    // Left alone, a fence falls over and stops blocking.
    hold(&mut w, Item::object("Dev_WoodFence", 2));
    assert!(w.place_current(t));
    if let Some(o) = w.location_mut().state.objects.get_mut(&t) {
        o.kind = ObjKind::Fence { health: 1, open: false };
    }
    w.end_day(false);
    assert!(matches!(w.locations["Farm"].object(t).unwrap().kind, ObjKind::Fence { health: 0, .. }));
    assert!(valley_sim::collision::tile_passable(&w.locations["Farm"], t));
}

#[test]
fn gates_open_and_close() {
    let mut w = world();
    hold(&mut w, Item::object("Dev_Gate", 1));
    let t = TilePos::new(20, 21);
    assert!(w.place_current(t));
    assert!(!valley_sim::collision::tile_passable(w.location(), t));
    assert!(w.do_action(Some(t)));
    assert!(valley_sim::collision::tile_passable(w.location(), t), "open gate");
    assert!(w.do_action(Some(t)));
    assert!(!valley_sim::collision::tile_passable(w.location(), t), "closed again");
    // Gates never wear out.
    for _ in 0..3 {
        w.end_day(false);
    }
    assert!(matches!(w.locations["Farm"].object(t).unwrap().kind, ObjKind::Fence { health: 1, .. }));
}

#[test]
fn paths_carry_objects_speed_up_the_farmer_and_lift_with_a_pickaxe() {
    let mut w = world();
    let t = TilePos::new(20, 21);
    hold(&mut w, Item::object("Dev_StonePath", 5));
    assert!(w.place_current(t));
    assert!(matches!(w.location().terrain(t), Some(TerrainFeature::Flooring(k)) if k == "Dev_StonePath"));
    assert_eq!(w.farmer.inventory.count("Dev_StonePath"), 4);
    assert!(!w.place_current(t), "one floor per tile");
    // A chest can stand on it.
    hold(&mut w, Item::big("Dev_Box", 1));
    assert!(w.place_current(t));
    w.location_mut().state.objects.remove(&t);
    // Standing on a path on the farm is quicker.
    w.farmer.set_tile(t);
    assert!(w.floor_speed_bonus() > 0.0);
    w.farmer.set_tile(TilePos::new(22, 22));
    assert_eq!(w.floor_speed_bonus(), 0.0);
    // The hoe can't dig it; the pickaxe lifts it.
    w.apply_tool(ToolKind::Hoe, 0, t);
    assert!(matches!(w.location().terrain(t), Some(TerrainFeature::Flooring(_))));
    w.dropped.clear();
    w.apply_tool(ToolKind::Pickaxe, 0, t);
    assert!(w.location().terrain(t).is_none());
    assert!(w.dropped.iter().any(|d| d.item.object_id() == Some("Dev_StonePath")));
}
