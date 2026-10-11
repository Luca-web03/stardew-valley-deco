use valley_content::ContentManager;
use valley_sim::world::{NewGame, World, WorldEvent};

fn world() -> World {
    let mut cm = ContentManager::dev_only();
    World::new(&mut cm, NewGame::default())
}

fn kill(w: &mut World, name: &str, n: i32) {
    for i in 0..n {
        let info = w.monster_info[name].clone();
        let m = valley_sim::combat::Monster::new(1000 + i as u32, info, "Farm", valley_sim::TilePos::new(10, 10));
        w.monster_killed(m, false);
    }
}

#[test]
fn goals_count_kills_and_the_guild_pays_out() {
    let mut w = world();
    assert_eq!(w.monster_goals.len(), 3);
    kill(&mut w, "Dev_Blob", 9);
    assert!(!w.goal_complete("Dev_Blobs"));
    // Too early: nothing to claim.
    w.drain_events();
    assert!(w.run_action_pub("Port.GuildMaster"));
    assert_eq!(w.farmer.inventory.count("Dev_GuardRing"), 0);
    kill(&mut w, "Dev_Blob", 1);
    assert!(w.goal_complete("Dev_Blobs"));
    let board = w.goal_lines();
    assert!(board.iter().any(|(n, k, g, done)| n == "Mud Blobs" && *k == 10 && *g == 10 && *done));
    // The guild shop doesn't sell the ring yet.
    w.drain_events();
    w.open_shop("AdventureShop");
    let has_ring = |w: &mut World| {
        w.drain_events().into_iter().any(|e| matches!(e, WorldEvent::OpenShop(s) if s.entries.iter().any(|x| x.item.object_id() == Some("Dev_GuardRing") && x.price == 1500)))
    };
    assert!(!has_ring(&mut w));
    assert!(w.run_action_pub("Port.GuildMaster"));
    assert_eq!(w.farmer.inventory.count("Dev_GuardRing"), 1);
    assert!(w.farmer.mail_received.contains(&"Gil_Dev_Blobs".to_string()));
    // Only once.
    w.run_action_pub("Port.GuildMaster");
    assert_eq!(w.farmer.inventory.count("Dev_GuardRing"), 1);
    w.open_shop("AdventureShop");
    assert!(has_ring(&mut w));
}

#[test]
fn several_targets_share_a_goal_and_rewards_send_mail() {
    let mut w = world();
    kill(&mut w, "Dev_Larva", 8);
    kill(&mut w, "Dev_Moth", 7);
    assert!(w.goal_complete("Dev_Crawlers"));
    assert_eq!(w.farmer.monster_kills.get("Dev_Moth"), Some(&7));
    w.run_action_pub("Port.GuildMaster");
    assert_eq!(w.farmer.inventory.count("Dev_MagnetRing"), 1);
    assert!(w.farmer.mail_tomorrow.contains(&"Dev_GuildThanks".to_string()));
    // Custom reward flags.
    kill(&mut w, "Dev_Mummy", 20);
    w.run_action_pub("Port.GuildMaster");
    assert!(w.farmer.mail_received.contains(&"Dev_UndeadCleared".to_string()));
    // Kills persist through a save.
    let save = valley_sim::save::SaveData::capture(&w);
    let json = serde_json::to_string(&save).unwrap();
    let back: valley_sim::save::SaveData = serde_json::from_str(&json).unwrap();
    let mut cm = ContentManager::dev_only();
    let w2 = back.restore(&mut cm);
    assert_eq!(w2.farmer.monster_kills.get("Dev_Mummy"), Some(&20));
}
