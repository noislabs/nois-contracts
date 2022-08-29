use rand_chacha::{rand_core::SeedableRng, ChaCha8Rng};

#[inline]
pub fn make_prng(randomness: [u8; 32]) -> ChaCha8Rng {
    // See https://github.com/rust-random/rand/issues/932 for a discussion
    // around various options.
    ChaCha8Rng::from_seed(randomness)
}
