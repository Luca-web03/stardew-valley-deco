//! Casino games played with Qi coins (original designs): "Lucky Reels", a
//! three-reel machine, and "High-Low", guess whether the next card is
//! higher or lower to double the pot. Coins are bought with gold at a coin
//! counter. Payout tables are engine constants.

use engine::Rng;

use crate::world::{World, WorldEvent};

/// Reel symbols (drawn by the game): 0 leaf, 1 moon, 2 bell, 3 star, 4 gem.
pub const REEL_SYMBOLS: usize = 5;
/// Symbol weights (rarer symbols pay more).
const REEL_WEIGHTS: [u32; REEL_SYMBOLS] = [40, 25, 18, 12, 5];
/// Payout multipliers for three of a kind, by symbol.
pub const TRIPLE_PAYOUT: [i32; REEL_SYMBOLS] = [3, 5, 8, 15, 50];
/// Any two leaves (the most common symbol) return the bet.
pub const PAIR_LEAF_PAYOUT: i32 = 1;
/// Coins per purchase at the counter and its price in gold.
pub const COIN_PACK: i32 = 100;
pub const COIN_PACK_PRICE: i32 = 1000;

/// Result of one spin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Spin {
    pub reels: [usize; 3],
    /// Coins won (0 if none); the bet is already paid.
    pub won: i32,
}

/// Payout for a set of reels and a bet.
pub fn spin_payout(reels: [usize; 3], bet: i32) -> i32 {
    if reels[0] == reels[1] && reels[1] == reels[2] {
        return bet * TRIPLE_PAYOUT[reels[0]];
    }
    if reels.iter().filter(|&&r| r == 0).count() == 2 {
        return bet * PAIR_LEAF_PAYOUT;
    }
    0
}

/// High-Low game state: the face-up card (1..=13) and the pot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HighLow {
    pub card: u8,
    pub pot: i32,
    pub streak: u32,
}

fn pick_symbol(rng: &mut Rng) -> usize {
    let total: u32 = REEL_WEIGHTS.iter().sum();
    let mut r = rng.range(0, total as i32) as u32;
    for (i, w) in REEL_WEIGHTS.iter().enumerate() {
        if r < *w {
            return i;
        }
        r -= w;
    }
    0
}

impl World {
    fn casino_rng(&mut self, salt: i64) -> Rng {
        self.casino_plays += 1;
        Rng::new(Rng::seed_from(&[self.seed as i64, self.clock.days_played() as i64, self.casino_plays as i64, salt]))
    }

    /// Buy a pack of Qi coins with gold.
    pub fn buy_coins(&mut self) -> bool {
        if self.farmer.money < COIN_PACK_PRICE {
            self.emit(WorldEvent::Message(format!("A pack of {COIN_PACK} coins costs {COIN_PACK_PRICE}g.")));
            return false;
        }
        self.farmer.money -= COIN_PACK_PRICE;
        self.farmer.qi_coins += COIN_PACK;
        self.sound("purchase");
        true
    }

    /// Spin the reels for `bet` coins.
    pub fn spin_reels(&mut self, bet: i32) -> Option<Spin> {
        if bet <= 0 || self.farmer.qi_coins < bet {
            return None;
        }
        self.farmer.qi_coins -= bet;
        let mut rng = self.casino_rng(1);
        let reels = [pick_symbol(&mut rng), pick_symbol(&mut rng), pick_symbol(&mut rng)];
        let won = spin_payout(reels, bet);
        self.farmer.qi_coins += won;
        self.sound(if won > 0 { "money" } else { "Cowboy_gunshot" });
        Some(Spin { reels, won })
    }

    /// Start a High-Low round with `bet` coins.
    pub fn highlow_start(&mut self, bet: i32) -> Option<HighLow> {
        if bet <= 0 || self.farmer.qi_coins < bet {
            return None;
        }
        self.farmer.qi_coins -= bet;
        let mut rng = self.casino_rng(2);
        Some(HighLow { card: rng.range(1, 14) as u8, pot: bet, streak: 0 })
    }

    /// Guess the next card; a tie counts as a loss. Returns the new state
    /// (pot doubled) or None when the guess was wrong (pot lost).
    pub fn highlow_guess(&mut self, game: HighLow, higher: bool) -> (u8, Option<HighLow>) {
        let mut rng = self.casino_rng(3);
        let next = rng.range(1, 14) as u8;
        let right = if higher { next > game.card } else { next < game.card };
        if right {
            self.sound("coin");
            (next, Some(HighLow { card: next, pot: game.pot * 2, streak: game.streak + 1 }))
        } else {
            self.sound("fishEscape");
            (next, None)
        }
    }

    /// Keep the pot.
    pub fn highlow_cash_out(&mut self, game: HighLow) {
        self.farmer.qi_coins += game.pot;
        self.sound("money");
    }
}
