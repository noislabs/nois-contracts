use rand_xoshiro::{rand_core::SeedableRng, Xoshiro256PlusPlus};

#[inline]
pub fn make_prng(randomness: [u8; 32]) -> Xoshiro256PlusPlus {
    // A PRNG that is not cryptographically secure.
    // See https://docs.rs/rand/0.8.5/rand/rngs/struct.SmallRng.html
    // where this is used for 32 bit systems.
    // We don't use the SmallRng in order to get the same implementation
    // in unit tests (64 bit dev machines) and the real contract (32 bit Wasm)

    // We chose the 256 bit variant as it allows using the full randomness value
    // but this might be overkill in out context. Maybe the 32bit version is better suited
    // for running in the wasm32 target.
    Xoshiro256PlusPlus::from_seed(randomness)
}
