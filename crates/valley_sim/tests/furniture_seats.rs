use valley_content::ContentManager;
use valley_sim::input::PlayerInput;
use valley_sim::item::Item;
use valley_sim::save::SaveData;
use valley_sim::world::{NewGame, World};
use valley_sim::{Dir, TilePos};

fn world() -> (ContentManager, World) {
    let mut cm = ContentManager::dev_only();
    let mut w = World::new(&mut cm, NewGame::default());
    w.event = None;
    (cm, w)
}

fn hold(w: &mut World, id: &str) {
    w.farmer.inventory.slots[11] = Some(Item::Furniture { id: id.into() });
    w.farmer.current_slot = 11;
}

#[test]
fn held_furniture_rotates_and_side_views_swap_the_footprint() {
    let (_, mut w) = world();
    let couch = w.items.furniture["Dev_Couch"].clone();
    assert_eq!(couch.rotations, 4);
    assert_eq!(couch.sprite_at(0, false), (0, 3, 2, false));
    assert_eq!(couch.sprite_at(1, false), (3, 1, 4, false));
    assert_eq!(couch.sprite_at(2, false), (4, 3, 2, false));
    assert_eq!(couch.sprite_at(3, false), (3, 1, 4, true));
    let chair = w.items.furniture["Dev_Chair"].clone();
    assert_eq!(chair.sprite_at(3, false), (1, 1, 2, true));
    assert_eq!(chair.footprint_at(1), (1, 1));
    w.farmer.set_tile(TilePos::new(5, 4));
    w.farmer.facing = Dir::Right;
    hold(&mut w, "Dev_Couch");
    assert!(w.do_action(None), "action turns it");
    assert_eq!(w.farmer.furniture_rotation, 1);
    assert!(w.begin_use(Some(TilePos::new(6, 4))), "use places it");
    let f = w.location().state.furniture.last().unwrap().clone();
    assert_eq!((f.tile, f.rotation, f.footprint), (TilePos::new(6, 4), 1, (1, 3)));
    assert_eq!(w.farmer.furniture_rotation, 0);
    assert!(!valley_sim::collision::tile_passable(w.location(), TilePos::new(6, 6)));
    // Saved with its rotation.
    let mut cm = ContentManager::dev_only();
    let r = SaveData::from_json(&SaveData::capture(&w).to_json()).unwrap().restore(&mut cm);
    assert_eq!(r.locations["FarmHouse"].state.furniture.last().unwrap().rotation, 1);
}

#[test]
fn farmers_sit_on_chairs_and_benches_and_get_up_by_moving() {
    let (_, mut w) = world();
    w.farmer.set_tile(TilePos::new(4, 4));
    w.farmer.facing = Dir::Down;
    hold(&mut w, "Dev_Chair");
    assert!(w.begin_use(Some(TilePos::new(4, 5))));
    w.farmer.current_slot = 30;
    let before = w.farmer.pos;
    assert!(w.do_action(Some(TilePos::new(4, 5))));
    assert!(w.is_sitting());
    assert_eq!(w.farmer.tile(), TilePos::new(4, 5));
    assert_eq!(w.farmer.facing, Dir::Down);
    // Stays put while idle.
    w.update(&PlayerInput::default());
    assert!(w.is_sitting());
    w.update(&PlayerInput { move_x: 1, ..Default::default() });
    assert!(!w.is_sitting());
    assert_eq!(w.farmer.pos, before);
    // A park bench on the town map (Data/ChairTiles).
    w.warp_farmer("Town", TilePos::new(12, 15), Some(Dir::Up));
    w.event = None;
    assert!(w.do_action(Some(TilePos::new(12, 14))));
    assert!(w.is_sitting());
    assert_eq!(w.farmer.facing, Dir::Down);
    w.warp_farmer("Farm", TilePos::new(12, 18), Some(Dir::Down));
    assert!(!w.is_sitting(), "warping stands you up");
}

#[test]
fn tables_hold_an_item_and_fireplaces_light_up() {
    let (_, mut w) = world();
    w.farmer.set_tile(TilePos::new(3, 4));
    w.farmer.facing = Dir::Down;
    hold(&mut w, "Dev_Table");
    assert!(w.begin_use(Some(TilePos::new(3, 5))));
    w.farmer.inventory.slots[12] = Some(Item::object("Dev_Turnip", 5));
    w.farmer.current_slot = 12;
    assert!(w.do_action(Some(TilePos::new(4, 5))));
    assert_eq!(w.farmer.inventory.count("Dev_Turnip"), 4);
    assert_eq!(w.location().state.furniture[0].held.as_ref().and_then(|i| i.object_id()), Some("Dev_Turnip"));
    // Picking it back up.
    assert!(w.do_action(Some(TilePos::new(3, 5))));
    assert_eq!(w.farmer.inventory.count("Dev_Turnip"), 5);
    assert!(w.location().state.furniture[0].held.is_none());
    // Fireplace.
    hold(&mut w, "Dev_Fireplace");
    w.farmer.set_tile(TilePos::new(7, 4));
    assert!(w.begin_use(Some(TilePos::new(7, 5))));
    assert!(w.furniture_lights("FarmHouse").is_empty());
    w.farmer.current_slot = 30;
    assert!(w.do_action(Some(TilePos::new(8, 5))));
    assert_eq!(w.furniture_lights("FarmHouse").len(), 1);
    assert_eq!(w.items.furniture["Dev_Fireplace"].sprite_at(0, true), (2, 2, 3, false));
    // Lamps glow after dusk.
    hold(&mut w, "Dev_Lamp");
    w.farmer.set_tile(TilePos::new(2, 7));
    assert!(w.begin_use(Some(TilePos::new(2, 8))));
    assert_eq!(w.furniture_lights("FarmHouse").len(), 1);
    w.clock.time = 2000;
    assert_eq!(w.furniture_lights("FarmHouse").len(), 2);
}

#[test]
fn outdoor_furniture_follows_its_placement_restriction() {
    let (_, mut w) = world();
    w.warp_farmer("Farm", TilePos::new(12, 18), Some(Dir::Down));
    let farm = w.locations.get_mut("Farm").unwrap();
    farm.state.objects.remove(&TilePos::new(12, 19));
    farm.state.terrain.remove(&TilePos::new(12, 19));
    hold(&mut w, "Dev_Lantern");
    assert!(w.begin_use(Some(TilePos::new(12, 19))));
    assert_eq!(w.locations["Farm"].state.furniture.len(), 1);
    hold(&mut w, "Dev_Chair");
    w.begin_use(Some(TilePos::new(13, 19)));
    assert_eq!(w.locations["Farm"].state.furniture.len(), 1, "chairs stay indoors");
}
