use valley_content::ContentManager;
use valley_sim::item::{Item, Tool, ToolKind};
use valley_sim::world::{NewGame, World, WorldEvent};
use valley_sim::{Dir, TilePos};

#[test]
fn the_phone_calls_shops_with_live_answers() {
    let mut cm = ContentManager::dev_only();
    let mut w = World::new(&mut cm, NewGame::default());
    let phone = TilePos::new(1, 3);
    assert!(w.locations["FarmHouse"].object(phone).is_some_and(|o| o.item_id == "Dev_Telephone"));
    w.warp_farmer("FarmHouse", TilePos::new(2, 3), Some(Dir::Left));
    w.drain_events();
    assert!(w.do_action(Some(phone)));
    let options = w.drain_events().into_iter().find_map(|e| match e {
        WorldEvent::Choice { options, .. } => Some(options),
        _ => None,
    }).expect("a list of numbers");
    assert!(options.iter().any(|(id, _)| id == "phone:Blacksmith"));
    // The blacksmith knows about the tool on the anvil.
    let mut axe = Tool::new(ToolKind::Axe);
    axe.level = 1;
    w.farmer.tool_upgrade = Some((axe, 2));
    w.choose("phone:Blacksmith");
    let said = w.drain_events().into_iter().find_map(|e| match e {
        WorldEvent::Message(m) => Some(m),
        _ => None,
    }).unwrap();
    assert!(said.contains("2 days") && said.contains("Axe"), "{said}");
    // The store knows today's dish, matching its stock.
    w.choose("phone:SeedShop");
    let said = w.drain_events().into_iter().find_map(|e| match e {
        WorldEvent::Message(m) => Some(m),
        _ => None,
    }).unwrap();
    let dish = w.todays_dish().unwrap();
    assert!(said.contains(&dish), "{said}");
    w.open_shop("SeedShop");
    let stock = w.drain_events().into_iter().find_map(|e| match e {
        WorldEvent::OpenShop(s) => Some(s),
        _ => None,
    }).unwrap();
    assert!(stock.entries.iter().any(|e| w.items.display_name(&e.item) == dish), "the store sells {dish}");
    let _ = Item::object("388", 1);
}
