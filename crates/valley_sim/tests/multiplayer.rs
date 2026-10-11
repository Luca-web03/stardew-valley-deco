use valley_content::ContentManager;
use valley_sim::input::PlayerInput;
use valley_sim::item::Item;
use valley_sim::save::SaveData;
use valley_sim::world::{NewGame, World, WorldEvent};
use valley_sim::{Dir, TilePos};

fn world() -> (ContentManager, World) {
    let mut cm = ContentManager::dev_only();
    let w = World::new(&mut cm, NewGame::default());
    (cm, w)
}

#[test]
fn farmhands_move_and_act_independently() {
    let (_, mut w) = world();
    let i = w.add_farmhand("Robin2");
    assert_eq!(w.farmhands[i].current, "FarmHouse");
    assert_eq!(w.farmhands[i].farmer.name, "Robin2");
    // Send the farmhand outside; the host stays put.
    w.as_farmhand(i, |w| w.warp_farmer("Farm", TilePos::new(14, 12), Some(Dir::Right)));
    assert_eq!(w.farmhands[i].current, "Farm");
    assert_eq!(w.current, "FarmHouse");
    let host_pos = w.farmer.pos;
    let x0 = w.farmhands[i].farmer.pos.x;
    for _ in 0..30 {
        w.update(&PlayerInput::default());
        w.update_farmhand(i, &PlayerInput { move_x: 1, ..Default::default() });
    }
    assert!(w.farmhands[i].farmer.pos.x > x0 + 50.0, "the farmhand walked");
    assert_eq!(w.farmer.pos, host_pos, "the host didn't");
    // Their own inventory and events.
    w.as_farmhand(i, |w| w.give_item(Item::object("Dev_Berry", 3)));
    assert_eq!(w.farmhands[i].farmer.inventory.count("Dev_Berry"), 3);
    assert_eq!(w.farmer.inventory.count("Dev_Berry"), 0);
    let ev = w.drain_farmhand_events(i);
    assert!(ev.iter().any(|e| matches!(e, WorldEvent::Pickup(_))));
    assert!(!w.drain_events().iter().any(|e| matches!(e, WorldEvent::Pickup(_))), "host didn't get it");
}

#[test]
fn farmhand_work_changes_the_shared_world() {
    let (_, mut w) = world();
    let i = w.add_farmhand("Pat");
    let t = TilePos::new(14, 13);
    {
        let farm = w.locations.get_mut("Farm").unwrap();
        farm.state.objects.remove(&t);
        farm.state.terrain.remove(&t);
    }
    w.as_farmhand(i, |w| {
        w.warp_farmer("Farm", TilePos::new(14, 12), Some(Dir::Down));
        w.farmer.inventory.slots[0] = Some(Item::Tool(valley_sim::item::Tool::new(valley_sim::item::ToolKind::Hoe)));
        w.farmer.current_slot = 0;
        w.apply_tool(valley_sim::item::ToolKind::Hoe, 0, t);
    });
    assert!(w.locations["Farm"].terrain(t).is_some(), "tilled soil is shared");
}

#[test]
fn farmhands_rest_overnight_and_are_saved() {
    let (mut cm, mut w) = world();
    let i = w.add_farmhand("Sam2");
    w.farmhands[i].farmer.stamina = 1.0;
    w.as_farmhand(i, |w| w.warp_farmer("Town", TilePos::new(10, 15), None));
    w.end_day(false);
    let p = &w.farmhands[i];
    assert_eq!(p.current, "FarmHouse");
    assert_eq!(p.farmer.stamina, p.farmer.max_stamina as f32);
    let json = SaveData::capture(&w).to_json();
    let r = SaveData::from_json(&json).unwrap().restore(&mut cm);
    assert_eq!(r.farmhands.len(), 1);
    assert_eq!(r.farmhands[0].farmer.name, "Sam2");
}
