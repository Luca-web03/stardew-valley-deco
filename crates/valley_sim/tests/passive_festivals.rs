use valley_content::ContentManager;
use valley_sim::save::SaveData;
use valley_sim::time::Season;
use valley_sim::world::{NewGame, World, WorldEvent};
use valley_sim::TilePos;

fn world() -> (ContentManager, World) {
    let mut cm = ContentManager::dev_only();
    let w = World::new(&mut cm, NewGame::default());
    (cm, w)
}

fn has_stall(w: &World) -> bool {
    w.locations["Dev_Dunes"].action_at(TilePos::new(18, 13)).is_some_and(|a| a.contains("Dev_MarketStall"))
}

#[test]
fn market_swaps_the_map_only_during_the_festival() {
    let (mut cm, mut w) = world();
    assert!(w.passive_festivals.contains_key("Dev_LanternMarket"));
    assert!(w.passive_maps.contains_key("Dev_Dunes_Market"));
    assert!(!has_stall(&w));
    w.clock.season = Season::Summer;
    w.clock.day = 9;
    w.end_day(false);
    assert_eq!(w.clock.day, 10);
    assert_eq!(w.passive_festivals_today().len(), 1);
    assert!(has_stall(&w), "market map in place");
    assert!(w.drain_events().iter().any(|e| matches!(e, WorldEvent::Hud(m) if m.contains("Lantern Market"))));
    // Still there after a save and load mid-festival.
    let json = SaveData::capture(&w).to_json();
    let r = SaveData::from_json(&json).unwrap().restore(&mut cm);
    assert!(has_stall(&r));
    for _ in 0..3 {
        w.end_day(false);
    }
    assert_eq!(w.clock.day, 13);
    assert!(w.passive_festivals_today().is_empty());
    assert!(!has_stall(&w), "the normal dunes are back");
}
