/// Small deterministic PRNG (xoshiro128** seeded via splitmix64).
///
/// Games derive per-day / per-feature seeds from it so that simulations are
/// reproducible in tests and replays.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rng {
    s: [u32; 4],
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        let mut z = seed;
        let mut next = || {
            z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut x = z;
            x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            x ^ (x >> 31)
        };
        let a = next();
        let b = next();
        let mut s = [a as u32, (a >> 32) as u32, b as u32, (b >> 32) as u32];
        if s == [0; 4] {
            s[0] = 1;
        }
        Self { s }
    }

    /// Combine several values into one seed (order sensitive).
    pub fn seed_from(parts: &[i64]) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for p in parts {
            for b in p.to_le_bytes() {
                h ^= b as u64;
                h = h.wrapping_mul(0x0100_0000_01b3);
            }
        }
        h
    }

    pub fn next_u32(&mut self) -> u32 {
        let result = self.s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 9;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(11);
        result
    }

    /// Uniform float in [0, 1).
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u32() >> 8) as f64 / (1u32 << 24) as f64
    }

    /// Uniform integer in [lo, hi). Returns `lo` if the range is empty.
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        let span = (hi as i64 - lo as i64) as u64;
        (lo as i64 + (self.next_u32() as u64 % span) as i64) as i32
    }

    pub fn chance(&mut self, p: f64) -> bool {
        self.next_f64() < p
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        if items.is_empty() {
            None
        } else {
            Some(&items[self.range(0, items.len() as i32) as usize])
        }
    }

    pub fn state(&self) -> [u32; 4] {
        self.s
    }

    pub fn from_state(s: [u32; 4]) -> Self {
        Self { s }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_in_range() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..1000 {
            let x = a.range(-5, 7);
            assert_eq!(x, b.range(-5, 7));
            assert!((-5..7).contains(&x));
            let f = a.next_f64();
            b.next_f64();
            assert!((0.0..1.0).contains(&f));
        }
    }
}
