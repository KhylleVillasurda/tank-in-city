//! Seeded RNG. Spawns and AI decisions come from here so that the same seed
//! plus the same inputs replays identically on WASM and on native (plan
//! section 4, "Seeded RNG" / "Input replay").

/// xorshift64. Small, dependency-free, and bit-identical everywhere.
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u32) -> Self {
        // A bare `seed | 1` collapses every even seed onto its odd neighbour,
        // so two runs seeded 2 and 3 would replay identically. Scramble first
        // (SplitMix64's finalizer), which also keeps the state away from the
        // zero fixed point xorshift would sit on.
        let mut z = (seed as u64).wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        Self {
            state: (z ^ (z >> 31)) | 1,
        }
    }

    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        (x >> 32) as u32
    }

    /// Uniform in `0..n`. `n == 0` returns 0.
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 {
            0
        } else {
            self.next_u32() % n
        }
    }

    /// Uniform in `lo..=hi`. Inclusive, and safe when `lo >= hi`.
    pub fn range(&mut self, lo: u32, hi: u32) -> u32 {
        if lo >= hi {
            lo
        } else {
            lo + self.below(hi - lo + 1)
        }
    }
}
