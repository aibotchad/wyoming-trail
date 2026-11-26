//! Seedable PRNG wrapper.
//!
//! `GameState` stores only the seed and day; each day derives a fresh
//! deterministic stream via [`Rng::for_day`], so full game state serializes
//! cleanly and runs are reproducible in tests.

use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};

#[derive(Debug, Clone)]
#[allow(dead_code)] // exercised by tests; first gameplay consumer: M2 (road events).
pub struct Rng {
    engine: StdRng,
}

#[allow(dead_code)] // exercised by tests; first gameplay consumer: M2 (road events).
impl Rng {
    /// Derive the day-specific stream from the run seed.
    pub fn for_day(seed: u64, day: u32) -> Self {
        let mixed = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (u64::from(day).rotate_left(7));
        Self {
            engine: StdRng::seed_from_u64(mixed),
        }
    }

    /// Uniform float in [0, 1).
    pub fn unit(&mut self) -> f64 {
        let v = self.engine.next_u64();
        // Top 32 bits of a 64-bit LCG-style source are well mixed enough for
        // game use; keeps the distribution stable across platforms.
        (v >> 32) as f64 / ((1u64 << 32) as f64)
    }

    /// Random pick from a slice (cloned). `None` for empty slices.
    pub fn choose<T: Copy>(&mut self, items: &[T]) -> Option<T> {
        if items.is_empty() {
            return None;
        }
        let idx = (self.unit() * items.len() as f64) as usize;
        Some(*items.get(idx.min(items.len() - 1))?)
    }

    /// True with the given probability (clamped to 0..=1).
    pub fn chance(&mut self, p: f64) -> bool {
        self.unit() < p.clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_and_day_give_same_stream() {
        let mut a = Rng::for_day(42, 3);
        let mut b = Rng::for_day(42, 3);
        for _ in 0..16 {
            assert_eq!(a.unit(), b.unit());
        }
    }

    #[test]
    fn different_days_diverge() {
        let mut a = Rng::for_day(42, 3);
        let mut b = Rng::for_day(42, 4);
        let (x, y) = (a.unit(), b.unit());
        assert_ne!(x, y);
    }

    #[test]
    fn choose_is_in_bounds() {
        let mut r = Rng::for_day(1, 1);
        let items = [10u32, 20, 30];
        for _ in 0..50 {
            let v = r.choose(&items);
            assert!(v.is_some() && items.contains(&v.unwrap()));
        }
        assert!(r.choose(&[] as &[u32]).is_none());
    }
}
