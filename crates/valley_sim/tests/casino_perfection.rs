use valley_content::ContentManager;
use valley_sim::casino::{spin_payout, COIN_PACK, COIN_PACK_PRICE, TRIPLE_PAYOUT};
use valley_sim::world::{NewGame, World, WorldEvent};
use valley_sim::{Dir, TilePos};

fn world() -> World {
    let mut cm = ContentManager::dev_only();
    World::new(&mut cm, NewGame::default())
}

#[test]
fn payout_table() {
    assert_eq!(spin_payout([4, 4, 4], 10), 10 * TRIPLE_PAYOUT[4]);
    assert_eq!(spin_payout([0, 0, 3], 10), 10);
    assert_eq!(spin_payout([1, 2, 3], 10), 0);
}

#[test]
fn coins_reels_and_high_low() {
    let mut w = world();
    assert!(w.spin_reels(10).is_none(), "no coins yet");
    w.farmer.money = 5000;
    assert!(w.buy_coins());
    assert_eq!((w.farmer.qi_coins, w.farmer.money), (COIN_PACK, 5000 - COIN_PACK_PRICE));
    // Many spins: the house usually wins but some spins pay out.
    w.farmer.qi_coins = 100_000;
    let mut wins = 0;
    for _ in 0..500 {
        let s = w.spin_reels(10).unwrap();
        assert_eq!(s.won, spin_payout(s.reels, 10));
        if s.won > 0 {
            wins += 1;
        }
    }
    assert!(wins > 20 && wins < 400, "wins {wins}");
    // High-low: a right guess doubles the pot, cashing out pays it.
    let mut g = w.highlow_start(10).unwrap();
    for _ in 0..20 {
        let (_, next) = w.highlow_guess(g, g.card <= 7);
        match next {
            Some(n) => {
                assert_eq!(n.pot, g.pot * 2);
                g = n;
                let coins = w.farmer.qi_coins;
                w.highlow_cash_out(g);
                assert_eq!(w.farmer.qi_coins, coins + g.pot);
                return;
            }
            None => g = w.highlow_start(10).unwrap(),
        }
    }
    panic!("never won a guess in 20 tries");
}

#[test]
fn parlor_tables_open_the_games() {
    let mut w = world();
    w.warp_farmer("Dev_Parlor", TilePos::new(2, 5), Some(Dir::Up));
    w.drain_events();
    assert!(w.do_action(Some(TilePos::new(2, 4))));
    assert!(w.drain_events().iter().any(|e| matches!(e, WorldEvent::OpenReels)));
    w.farmer.set_tile(TilePos::new(5, 5));
    assert!(w.do_action(Some(TilePos::new(5, 4))));
    assert!(w.drain_events().iter().any(|e| matches!(e, WorldEvent::OpenHighLow)));
}

#[test]
fn perfection_grows_with_progress() {
    let mut w = world();
    let start = w.perfection();
    assert!((0.0..20.0).contains(&start), "{start}");
    for id in w.items.objects.keys().cloned().collect::<Vec<_>>() {
        w.farmer.shipped.insert(id, 1);
    }
    let after = w.perfection();
    assert!(after >= start + 14.0, "shipping everything adds ~15% ({start} -> {after})");
    assert!(after <= 100.0);
}
