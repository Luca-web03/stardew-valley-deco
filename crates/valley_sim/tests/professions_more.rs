use valley_content::ContentManager;
use valley_sim::item::Item;
use valley_sim::mines::RockKind;
use valley_sim::objects::{ObjKind, PlacedObject};
use valley_sim::world::{NewGame, World};
use valley_sim::TilePos;

#[test]
fn gem_nodes_appear_and_geologists_find_pairs() {
    let mut cm = ContentManager::dev_only();
    let mut w = World::new(&mut cm, NewGame::default());
    // Somewhere in the first floors a gem node turns up.
    let mut found = None;
    for level in 1..30 {
        w.enter_mine_level(level);
        if let Some((p, _)) = w.location().state.objects.iter().find(|(_, o)| matches!(o.kind, ObjKind::MineRock { rock: RockKind::Gem(_), .. })) {
            found = Some(*p);
            break;
        }
    }
    let p = found.expect("a gem node in the early mines");
    w.farmer.professions.push("geologist".into());
    let mut pairs = 0;
    for i in 0..20 {
        let t = TilePos::new(p.x, p.y);
        w.location_mut().state.objects.insert(t, PlacedObject::new("Dev_GemNode", false, ObjKind::MineRock { rock: RockKind::Gem((i % 2) as u8), health: 1 }));
        w.dropped.clear();
        w.game_ms += 977;
        assert!(w.hit_mine_rock_pub(t, 10));
        let gems: Vec<&Item> = w.dropped.iter().map(|d| &d.item).filter(|i| i.object_id().is_some_and(|id| id == "Dev_Amethyst" || id == "Dev_Topaz")).collect();
        assert_eq!(gems.len(), 1);
        if gems[0].stack() == 2 {
            pairs += 1;
        }
    }
    assert!(pairs > 2 && pairs < 18, "{pairs} pairs");
}

#[test]
fn blacksmiths_sell_bars_for_more() {
    let mut cm = ContentManager::dev_only();
    let mut w = World::new(&mut cm, NewGame::default());
    let bar = Item::object("Dev_IronBar", 1);
    let before = w.sell_value(&bar);
    w.farmer.professions.push("blacksmith".into());
    assert_eq!(w.sell_value(&bar), (before as f32 * 1.5) as i32);
}
