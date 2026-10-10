use valley_content::ContentManager;
use valley_sim::children::{ChildStage, DAYS_UNTIL_BIRTH};
use valley_sim::farmer::RelationStatus;
use valley_sim::save::SaveData;
use valley_sim::world::{NewGame, World, WorldEvent};
use valley_sim::{Dir, TilePos};

fn world() -> (ContentManager, World) {
    let mut cm = ContentManager::dev_only();
    let w = World::new(&mut cm, NewGame::default());
    (cm, w)
}

/// Married to Mara with a big enough house and lots of friendship.
fn married(w: &mut World) -> String {
    let spouse = "Mara".to_string();
    let f = w.farmer.friendship(&spouse);
    f.points = 3000;
    f.status = RelationStatus::Married;
    w.farmer.spouse = Some(spouse.clone());
    w.farmer.house_level = 1;
    w.end_day(false);
    spouse
}

fn talk_to_spouse(w: &mut World, spouse: &str) -> Vec<WorldEvent> {
    let i = w.npcs.iter().position(|n| n.name == spouse).unwrap();
    let t = w.npcs[i].tile();
    w.warp_farmer("FarmHouse", TilePos::new(t.x, t.y + 1), Some(Dir::Up));
    w.npcs[i].set_tile(t);
    w.drain_events();
    w.do_action(Some(t));
    w.drain_events()
}

#[test]
fn spouse_asks_and_a_baby_arrives_two_weeks_later() {
    let (_, mut w) = world();
    let spouse = married(&mut w);
    assert_eq!(w.npcs.iter().find(|n| n.name == spouse).unwrap().location, "FarmHouse");
    w.child_question_pending = true;
    let ev = talk_to_spouse(&mut w, &spouse);
    assert!(ev.iter().any(|e| matches!(e, WorldEvent::DialogueQuestion { .. })), "the spouse asks");
    w.answer_question(0);
    let due = w.farmer.baby_due.expect("baby on the way");
    assert_eq!(due, w.clock.days_played() + DAYS_UNTIL_BIRTH);
    assert!(!w.farmer.dialogue_answers.iter().any(|a| a.starts_with("__")));
    for _ in 0..DAYS_UNTIL_BIRTH {
        assert!(w.farmer.children.is_empty());
        w.end_day(false);
    }
    assert_eq!(w.farmer.children.len(), 1);
    assert!(w.farmer.baby_due.is_none());
    let today = w.clock.days_played();
    assert_eq!(w.farmer.children[0].stage(today), ChildStage::Newborn);
    let crib = w.crib_tile();
    assert!(valley_sim::collision::tile_passable(&w.locations["FarmHouse"], crib));
}

#[test]
fn saying_no_or_unmet_conditions_mean_no_baby() {
    let (_, mut w) = world();
    let spouse = married(&mut w);
    w.child_question_pending = true;
    talk_to_spouse(&mut w, &spouse);
    w.answer_question(1);
    assert!(w.farmer.baby_due.is_none());
    // Without a nursery-sized house the spouse never asks.
    w.farmer.house_level = 0;
    for _ in 0..60 {
        w.end_day(false);
        assert!(!w.child_question_pending);
    }
}

#[test]
fn children_grow_up_and_toddlers_wander() {
    let (_, mut w) = world();
    married(&mut w);
    w.farmer.baby_due = Some(w.clock.days_played() + 1);
    w.end_day(false);
    assert_eq!(w.farmer.children.len(), 1);
    let born = w.farmer.children[0].born;
    let stages: Vec<ChildStage> = [0, 20, 40, 60].iter().map(|d| w.farmer.children[0].stage(born + d)).collect();
    assert_eq!(stages, vec![ChildStage::Newborn, ChildStage::Sitter, ChildStage::Crawler, ChildStage::Toddler]);
    // Make them a toddler and let them walk around the house.
    w.farmer.children[0].born -= 70;
    w.end_day(false);
    w.warp_farmer("FarmHouse", w.wake_tile(), Some(Dir::Down));
    let start = w.farmer.children[0].pos;
    for _ in 0..600 {
        w.update(&valley_sim::input::PlayerInput::default());
    }
    assert_ne!(w.farmer.children[0].pos, start);
    // Talking raises friendship once a day.
    let c = w.farmer.children[0].pos;
    let ct = TilePos::from_world(valley_sim::farm::FarmAnimal::bbox_at(c).center());
    w.farmer.set_tile(TilePos::new(ct.x, ct.y + 1));
    assert!(w.do_action(Some(ct)));
    assert!(w.do_action(Some(ct)));
    assert_eq!(w.farmer.children[0].friendship, 20);
}

#[test]
fn children_survive_save() {
    let (mut cm, mut w) = world();
    married(&mut w);
    w.farmer.baby_due = Some(w.clock.days_played() + 1);
    w.end_day(false);
    let json = SaveData::capture(&w).to_json();
    let r = SaveData::from_json(&json).unwrap().restore(&mut cm);
    assert_eq!(r.farmer.children.len(), 1);
    assert_eq!(r.farmer.children[0].name, w.farmer.children[0].name);
}
