//! Long-running smoke test: a full year with most systems active, saving
//! and reloading every month. Catches panics and broken invariants that
//! only show up across seasons.

use valley_content::ContentManager;
use valley_sim::farmer::RelationStatus;
use valley_sim::input::PlayerInput;
use valley_sim::item::Item;
use valley_sim::save::SaveData;
use valley_sim::world::{NewGame, World};
use valley_sim::TilePos;

#[test]
fn a_busy_year_runs_and_saves_cleanly() {
    let mut cm = ContentManager::dev_only();
    let mut w = World::new(&mut cm, NewGame::default());
    w.farmer.money = 200_000;
    w.farmer.inventory.add(Item::object("388", 999));
    w.farmer.inventory.add(Item::object("390", 999));
    {
        let farm = w.locations.get_mut("Farm").unwrap();
        for y in 15..30 {
            for x in 5..40 {
                farm.state.objects.remove(&TilePos::new(x, y));
                farm.state.terrain.remove(&TilePos::new(x, y));
            }
        }
    }
    w.place_building("Coop", TilePos::new(6, 16)).unwrap();
    w.place_building("Dev_Pond", TilePos::new(12, 16)).unwrap();
    w.end_day(false);
    w.buy_animal("Dev_Hen", None).unwrap();
    // Marry Mara and set up a pending baby.
    {
        let f = w.farmer.friendship("Mara");
        f.points = 3000;
        f.status = RelationStatus::Married;
    }
    w.farmer.spouse = Some("Mara".into());
    w.farmer.baby_due = Some(w.clock.days_played() + 10);
    w.accept_order("Dev_FishFry").ok();

    let start_year = w.clock.year;
    for day in 0..112 {
        // A little play each day: walk around, use the held tool, talk.
        for t in 0..120 {
            let input = PlayerInput { move_x: [1, 0, -1, 0][(t / 30) % 4], move_y: [0, 1, 0, -1][(t / 30) % 4], use_tool: t % 40 == 0, ..Default::default() };
            w.update(&input);
        }
        w.drain_events();
        // Keep the shipping bin busy.
        if day % 3 == 0 {
            w.shipping_bin.push(Item::object("Dev_Turnip", 2));
        }
        w.end_day(false);
        w.drain_events();
        assert!(w.farmer.health >= 1 && w.farmer.money >= 0, "day {day}");
        assert!(w.farmer.inventory.slots.iter().flatten().all(|i| i.stack() > 0), "no empty stacks on day {day}");
        if day % 28 == 27 {
            let json = SaveData::capture(&w).to_json();
            let restored = SaveData::from_json(&json).expect("save parses").restore(&mut cm);
            assert_eq!(restored.clock.days_played(), w.clock.days_played());
            assert_eq!(restored.farmer.money, w.farmer.money);
            assert_eq!(restored.farmer.children.len(), w.farmer.children.len());
            w = restored;
        }
    }
    assert_eq!(w.clock.year, start_year + 1);
    assert_eq!(w.farmer.children.len(), 1);
    assert!(w.animals[0].age_days >= 100);
}
