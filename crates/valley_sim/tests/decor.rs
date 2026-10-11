use valley_content::ContentManager;
use valley_sim::item::Item;
use valley_sim::save::SaveData;
use valley_sim::world::{NewGame, World};
use valley_sim::{Dir, TilePos};

#[test]
fn wallpaper_and_flooring_paint_a_room_and_are_saved() {
    let mut cm = ContentManager::dev_only();
    let mut w = World::new(&mut cm, NewGame::default());
    assert_eq!(w.items.create("(WP)3", 1), Some(Item::Decor { id: "3".into(), floor: false }));
    assert!(w.items.create("(FL)99", 1).is_none(), "only 56 floors");
    w.warp_farmer("FarmHouse", TilePos::new(5, 6), Some(Dir::Up));
    let rooms = valley_sim::decor::rooms(w.location());
    let room = rooms.iter().find(|r| r.floor.contains(&TilePos::new(5, 6))).unwrap().clone();
    assert!(room.walls.iter().any(|(p, row)| *p == TilePos::new(5, 2) && *row == 0));
    // Wallpaper on a wall tile out of reach still works.
    w.farmer.inventory.slots[8] = Some(Item::Decor { id: "2".into(), floor: false });
    w.farmer.current_slot = 8;
    assert!(w.begin_use(Some(TilePos::new(8, 2))));
    assert_eq!(w.location().state.paint.wallpaper.get(&room.key).map(String::as_str), Some("2"));
    assert!(w.farmer.inventory.get(8).is_none(), "used up");
    w.farmer.inventory.slots[8] = Some(Item::Decor { id: "1".into(), floor: true });
    assert!(w.begin_use(Some(TilePos::new(3, 8))));
    assert_eq!(w.location().state.paint.flooring.get(&room.key).map(String::as_str), Some("1"));
    // Not outdoors.
    w.warp_farmer("Farm", TilePos::new(20, 20), Some(Dir::Down));
    w.farmer.inventory.slots[8] = Some(Item::Decor { id: "1".into(), floor: true });
    w.begin_use(Some(TilePos::new(20, 21)));
    assert!(w.farmer.inventory.get(8).is_some());
    let json = SaveData::capture(&w).to_json();
    let back = SaveData::from_json(&json).unwrap().restore(&mut cm);
    assert_eq!(back.locations["FarmHouse"].state.paint, w.locations["FarmHouse"].state.paint);
}
