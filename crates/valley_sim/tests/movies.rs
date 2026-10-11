use valley_content::ContentManager;
use valley_sim::item::Item;
use valley_sim::world::{NewGame, World, WorldEvent};
use valley_sim::{Dir, TilePos};

fn world() -> World {
    let mut cm = ContentManager::dev_only();
    World::new(&mut cm, NewGame::default())
}

fn theater_door() -> TilePos {
    TilePos::new(33, 8)
}

fn talk_to(w: &mut World, npc: &str) {
    let i = w.npcs.iter().position(|n| n.name == npc).unwrap();
    let loc = w.npcs[i].location.clone();
    let t = w.npcs[i].tile();
    w.warp_farmer(&loc, TilePos::new(t.x, t.y + 1), Some(Dir::Up));
    w.npcs[i].set_tile(t);
    w.drain_events();
    w.do_action(Some(t));
}

#[test]
fn seasonal_movie_and_solo_ticket() {
    let mut w = world();
    assert_eq!(w.movie_today().unwrap().id, "Dev_LanternFlight");
    w.warp_farmer("Town", TilePos::new(33, 9), Some(Dir::Up));
    w.drain_events();
    assert!(w.do_action(Some(theater_door())));
    assert!(w.drain_events().iter().any(|e| matches!(e, WorldEvent::Message(m) if m.contains("need a ticket"))));
    w.farmer.inventory.add(Item::object("Dev_Ticket", 1));
    assert!(w.do_action(Some(theater_door())));
    let pages = w.drain_events().into_iter().find_map(|e| match e {
        WorldEvent::Dialogue { pages, .. } => Some(pages),
        _ => None,
    });
    let pages = pages.expect("the movie plays");
    assert!(pages[0].text.contains("Lantern Flight"));
    assert_eq!(pages.len(), 4);
    assert_eq!(w.farmer.inventory.count("Dev_Ticket"), 0);
    assert_eq!(w.farmer.movies_seen, vec!["Dev_LanternFlight"]);
}

#[test]
fn inviting_a_villager_changes_friendship_by_taste() {
    let mut w = world();
    w.farmer.inventory.slots[8] = Some(Item::object("Dev_Ticket", 2));
    w.farmer.current_slot = 8;
    talk_to(&mut w, "Mara");
    assert_eq!(w.movie_invite.as_deref(), Some("Mara"));
    assert_eq!(w.farmer.inventory.count("Dev_Ticket"), 1);
    let before = w.farmer.friendships.get("Mara").map(|f| f.points).unwrap_or(0);
    w.warp_farmer("Town", TilePos::new(33, 9), Some(Dir::Up));
    w.drain_events();
    assert!(w.do_action(Some(theater_door())));
    // A snack is offered first; skip it.
    let offered = w.drain_events().iter().any(|e| matches!(e, WorldEvent::Choice { options, .. } if options.iter().any(|(id, _)| id == "concession:")));
    assert!(offered);
    assert_eq!(w.movie_invite.as_deref(), Some("Mara"), "still invited until the movie starts");
    w.choose("concession:");
    let after = w.farmer.friendships["Mara"].points;
    assert_eq!(after - before, 200, "Mara loves adventure films");
    assert_eq!(w.farmer.inventory.count("Dev_Ticket"), 1, "the invitation covered the showing");
    assert!(w.movie_invite.is_none());
    // Fall brings a drama, which Mara dislikes and Tobin likes.
    w.clock.season = valley_sim::time::Season::Fall;
    let movie = w.movie_today().unwrap().clone();
    assert_eq!(movie.id, "Dev_IronWinter");
    assert_eq!(w.movie_reaction("Mara", &movie), "dislike");
    assert_eq!(w.movie_reaction("Tobin", &movie), "like");
    assert_eq!(w.movie_reaction("Nobody", &movie), "neutral");
}

#[test]
fn a_snack_the_guest_loves_adds_friendship() {
    let mut w = world();
    w.farmer.inventory.slots[8] = Some(Item::object("Dev_Ticket", 1));
    w.farmer.current_slot = 8;
    talk_to(&mut w, "Mara");
    let before = w.farmer.friendships.get("Mara").map(|f| f.points).unwrap_or(0);
    let money = w.farmer.money;
    w.warp_farmer("Town", TilePos::new(33, 9), Some(Dir::Up));
    assert!(w.do_action(Some(theater_door())));
    w.choose("concession:Dev_Pretzel");
    assert_eq!(w.farmer.money, money - 60);
    assert_eq!(w.farmer.friendships["Mara"].points - before, 200 + 50, "loves the movie and the hearty pretzel");
    let pretzel = w.concessions.iter().find(|c| c.id == "Dev_Pretzel").unwrap().clone();
    let pop = w.concessions.iter().find(|c| c.id == "Dev_FizzyPop").unwrap().clone();
    assert_eq!(w.concession_taste("Mara", &pretzel), "love");
    assert_eq!(w.concession_taste("Mara", &pop), "dislike");
    assert_eq!(w.concession_taste("Tobin", &pop), "like", "everyone else likes sweets");
}
