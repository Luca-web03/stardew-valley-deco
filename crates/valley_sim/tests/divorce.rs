use valley_content::ContentManager;
use valley_sim::farmer::RelationStatus;
use valley_sim::item::Item;
use valley_sim::world::{NewGame, World, WorldEvent};
use valley_sim::{Dir, TilePos};

fn married_world() -> World {
    let mut cm = ContentManager::dev_only();
    let mut w = World::new(&mut cm, NewGame::default());
    let f = w.farmer.friendship("Mara");
    f.points = 3000;
    f.status = RelationStatus::Married;
    w.farmer.spouse = Some("Mara".into());
    w.end_day(false);
    w
}

#[test]
fn divorce_costs_money_and_the_spouse_moves_out() {
    let mut w = married_world();
    let mara = w.npcs.iter().position(|n| n.name == "Mara").unwrap();
    assert_eq!(w.npcs[mara].location, "FarmHouse");
    w.warp_farmer("Town", TilePos::new(27, 12), Some(Dir::Up));
    w.drain_events();
    assert!(w.do_action(Some(TilePos::new(27, 11))));
    assert!(w.drain_events().iter().any(|e| matches!(e, WorldEvent::DivorcePrompt)));
    w.farmer.money = 100;
    assert!(w.divorce().is_err(), "can't afford the fee");
    w.farmer.money = 60_000;
    assert!(w.divorce().is_ok());
    assert_eq!(w.farmer.money, 10_000);
    assert!(w.farmer.spouse.is_none());
    assert_eq!(w.farmer.friendships["Mara"].status, RelationStatus::Divorced);
    assert_eq!(w.farmer.friendships["Mara"].points, 0);
    w.end_day(false);
    let home = w.npcs[mara].original_home.clone();
    assert_ne!(home.location, "FarmHouse");
    assert_eq!(w.npcs[mara].location, home.location, "back home");
    // No second chances with a bouquet.
    w.farmer.friendship("Mara").points = 2500;
    let i = mara;
    let t = w.npcs[i].tile();
    w.warp_farmer(&w.npcs[i].location.clone(), TilePos::new(t.x, t.y + 1), Some(Dir::Up));
    w.farmer.inventory.slots[3] = Some(Item::object("Dev_Bouquet", 1));
    w.farmer.current_slot = 3;
    w.drain_events();
    w.do_action(Some(t));
    assert_eq!(w.farmer.friendships["Mara"].status, RelationStatus::Divorced);
    assert_eq!(w.farmer.inventory.count("Dev_Bouquet"), 1);
}

#[test]
fn gifts_to_a_rival_in_front_of_the_spouse_cost_friendship() {
    let mut w = married_world();
    // Make Tobin romanceable for the test and put both villagers side by side.
    let tobin = w.npcs.iter().position(|n| n.name == "Tobin").unwrap();
    let mara = w.npcs.iter().position(|n| n.name == "Mara").unwrap();
    w.npcs[tobin].can_romance = true;
    w.warp_farmer("Town", TilePos::new(10, 15), Some(Dir::Up));
    w.npcs[tobin].location = "Town".into();
    w.npcs[tobin].set_tile(TilePos::new(10, 14));
    w.npcs[mara].location = "Town".into();
    w.npcs[mara].set_tile(TilePos::new(12, 15));
    let before = w.farmer.friendships["Mara"].points;
    // Talk first (gifts follow a chat), then give.
    w.do_action(Some(TilePos::new(10, 14)));
    w.farmer.inventory.slots[3] = Some(Item::object("Dev_Berry", 1));
    w.farmer.current_slot = 3;
    w.npcs[tobin].pause_ms = 0;
    w.do_action(Some(TilePos::new(10, 14)));
    assert_eq!(w.farmer.inventory.count("Dev_Berry"), 0, "the gift was given");
    assert_eq!(w.farmer.friendships["Mara"].points, before - 30);
}
