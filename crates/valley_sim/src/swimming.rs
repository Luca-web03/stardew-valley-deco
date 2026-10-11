//! Swimming and the spa (public behaviour; rates approx.). `TouchAction`
//! tiles: `ChangeIntoSwimsuit` / `ChangeOutOfSwimsuit` (locker rooms) and
//! `PoolEntrance` (step in or out of the water; needs the swimsuit). While
//! swimming the farmer moves only through water and pool entrances. In a
//! spa pool (map property `Port.Spa`, or a location named
//! `BathHouse_Pool`) swimming restores energy and health.

use engine::IRect;

use crate::world::World;
use crate::{TilePos, TILE};

/// Milliseconds per point of energy and health restored in the spa.
pub const SPA_TICK_MS: u32 = 200;

impl World {
    pub fn is_spa(&self) -> bool {
        self.current == "BathHouse_Pool" || self.location().map.prop("Port.Spa").is_some()
    }

    /// Touch actions other than warps, when the feet enter a new tile.
    pub(crate) fn touch_action(&mut self, action: &str, at: TilePos) {
        let toks: Vec<&str> = action.split_whitespace().collect();
        match toks.first().copied() {
            Some("ChangeIntoSwimsuit") => {
                if !self.farmer.swimsuit {
                    self.farmer.swimsuit = true;
                    self.sound("dwop");
                }
            }
            Some("ChangeOutOfSwimsuit") => {
                if self.farmer.swimsuit {
                    self.farmer.swimsuit = false;
                    self.farmer.swimming = false;
                    self.sound("dwop");
                }
            }
            Some("PoolEntrance") => {
                if !self.farmer.swimsuit {
                    self.hud("You need to be wearing your swimsuit.");
                    return;
                }
                self.farmer.swimming = !self.farmer.swimming;
                if self.farmer.swimming {
                    // Hop forward into the water.
                    let n = at.offset(self.farmer.facing);
                    if self.location().is_water(n) {
                        self.farmer.set_tile(n);
                    }
                    self.sound("pullItemFromWater");
                } else {
                    self.sound("dwop");
                }
                self.last_touch = Some(self.farmer.tile());
            }
            _ => {}
        }
    }

    /// While swimming, only water and pool entrances can be entered.
    pub(crate) fn swim_blocked(&self, bb: IRect) -> bool {
        let loc = self.location();
        let (x0, y0) = (bb.x.div_euclid(TILE), bb.y.div_euclid(TILE));
        let (x1, y1) = ((bb.right() - 1).div_euclid(TILE), (bb.bottom() - 1).div_euclid(TILE));
        for y in y0..=y1 {
            for x in x0..=x1 {
                let p = TilePos::new(x, y);
                let entrance = loc.touch_action_at(p).is_some_and(|a| a.starts_with("PoolEntrance"));
                if !loc.is_water(p) && !entrance {
                    return true;
                }
            }
        }
        false
    }

    /// Per tick: spa pools restore energy and health.
    pub(crate) fn update_swimming(&mut self, dt: u32) {
        if !self.farmer.swimming {
            self.spa_ms = 0;
            return;
        }
        if !self.is_spa() {
            return;
        }
        self.spa_ms += dt;
        while self.spa_ms >= SPA_TICK_MS {
            self.spa_ms -= SPA_TICK_MS;
            let f = &mut self.farmer;
            f.stamina = (f.stamina + 1.0).min(f.max_stamina as f32);
            f.health = (f.health + 1).min(f.max_health);
            if f.stamina > 0.0 {
                f.exhausted = false;
            }
        }
    }
}
