use valley_content::data::{MachineData, MachineOutputItem, MachineOutputRule, MachineTrigger, QuantityModifier};
use valley_content::ContentManager;
use valley_sim::item::Item;
use valley_sim::objects::{ObjKind, PlacedObject};
use valley_sim::world::{NewGame, World};
use valley_sim::{Dir, TilePos};

fn rule(id: &str, item: &str, count: i32, out: MachineOutputItem, minutes: i32) -> MachineOutputRule {
    MachineOutputRule {
        id: id.into(),
        triggers: vec![MachineTrigger { trigger: "ItemPlacedInMachine".into(), required_item_id: Some(item.into()), required_count: count, ..Default::default() }],
        output_item: vec![out],
        minutes_until_ready: minutes,
        days_until_ready: -1,
        invalid_count_message: Some("Needs {0}!".into()),
    }
}

#[test]
fn machine_rules_modifiers_messages_and_xp() {
    let mut cm = ContentManager::dev_only();
    let mut w = World::new(&mut cm, NewGame::default());
    w.warp_farmer("Farm", TilePos::new(20, 20), Some(Dir::Down));
    let t = TilePos::new(20, 21);
    let out = MachineOutputItem { item_id: Some("(O)Dev_Juice".into()), min_stack: 1, max_stack: 1, stack_modifiers: Some(vec![QuantityModifier { modification: "Add".into(), amount: 2.0, ..Default::default() }]), ..Default::default() };
    let data = MachineData {
        output_rules: vec![rule("r", "(O)Dev_Turnip", 3, out, 60)],
        ready_time_modifiers: Some(vec![QuantityModifier { modification: "Multiply".into(), amount: 0.5, condition: Some("SEASON Spring".into()), ..Default::default() }]),
        experience_gain_on_harvest: Some("farming 7".into()),
        prevent_time_pass: Some(vec!["Winter".into()]),
        ..Default::default()
    };
    w.items.machines.insert("(BC)Dev_Juicer".into(), data);
    w.location_mut().state.objects.insert(t, PlacedObject::new("Dev_Juicer", true, ObjKind::Machine { output: None, minutes_left: 0, ready: false }));
    w.farmer.inventory.slots[8] = Some(Item::object("Dev_Turnip", 2));
    w.farmer.current_slot = 8;
    w.drain_events();
    assert!(w.do_action(Some(t)));
    assert_eq!(w.farmer.inventory.count("Dev_Turnip"), 2, "not enough turnips");
    assert!(w.drain_events().iter().any(|e| matches!(e, valley_sim::world::WorldEvent::Hud(m) if m == "Needs 3!")));
    w.farmer.inventory.slots[8] = Some(Item::object("Dev_Turnip", 3));
    assert!(w.do_action(Some(t)));
    let ObjKind::Machine { minutes_left, output: Some(o), .. } = &w.location().object(t).unwrap().kind else { panic!() };
    assert_eq!(*minutes_left, 30, "spring halves the time");
    assert_eq!(o.stack(), 3, "stack modifier");
    w.advance_machines(30);
    let xp = w.farmer.skills.xp(valley_sim::farmer::Skill::Farming);
    assert!(w.do_action(Some(t)));
    assert_eq!(w.farmer.skills.xp(valley_sim::farmer::Skill::Farming), xp + 7);
}

#[test]
fn geode_crushers_open_geodes() {
    let mut cm = ContentManager::dev_only();
    let w = World::new(&mut cm, NewGame::default());
    let mut rng = engine::Rng::new(5);
    let found = w.geode_contents("Dev_Geode", &mut rng).unwrap();
    assert!(w.items.objects.contains_key(found.object_id().unwrap()));
    // Data-defined drops win.
    let mut w = w;
    if let Some(o) = w.items.objects.get_mut("Dev_Geode") {
        o.data.geode_drops = vec![valley_content::data::GeodeDrop { item_id: Some("(O)Dev_Ruby".into()), chance: 1.0, min_stack: 2, max_stack: 2, ..Default::default() }];
    }
    let found = w.geode_contents("Dev_Geode", &mut rng).unwrap();
    assert_eq!((found.object_id(), found.stack()), (Some("Dev_Ruby"), 2));
    assert!(w.is_geode("Dev_Geode"));
}
