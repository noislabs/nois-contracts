use rand_xoshiro::{rand_core::RngCore, Xoshiro256PlusPlus};
use xxhash_rust::xxh3::xxh3_128;

use crate::prng::make_prng;

pub struct SubRandomnessProvider {
    rng: Xoshiro256PlusPlus,
}

impl SubRandomnessProvider {
    pub fn provide(&mut self) -> [u8; 32] {
        let mut out = [0u8; 32];
        self.rng.fill_bytes(&mut out);
        out
    }
}

impl Iterator for SubRandomnessProvider {
    type Item = [u8; 32];

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.provide())
    }
}

/// Takes a randomness and a key. Returns an arbitrary number of sub-randomnesses.
/// The key is mixed into the randomness such that calling this function with different keys
/// leads to different outputs. Calling it with the same key and randomness leads to the same outputs.
pub fn sub_randomness_with_key(
    mut randomness: [u8; 32],
    key: impl AsRef<[u8]>,
) -> Box<SubRandomnessProvider> {
    let hashed_key = xxh3_128(key.as_ref()).to_be_bytes();
    for (pos, byte) in hashed_key.iter().enumerate() {
        randomness[pos] ^= byte;
    }

    let rng = make_prng(randomness);

    Box::new(SubRandomnessProvider { rng })
}

/// Takes a randomness and a key. Returns an arbitrary number of sub-randomnesses.
///
/// This is equivalent to calling [`sub_randomness_with_key`] with key "_^default^_"
pub fn sub_randomness(randomness: [u8; 32]) -> Box<SubRandomnessProvider> {
    sub_randomness_with_key(randomness, b"_^default^_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sub_randomness_with_key_works() {
        // outputs are the same for the same randomness and key
        let mut provider1 = sub_randomness_with_key([0xA6; 32], "A");
        let mut provider2 = sub_randomness_with_key([0xA6; 32], "A");
        assert_eq!(provider1.provide(), provider2.provide());
        assert_eq!(provider1.provide(), provider2.provide());
        assert_eq!(provider1.provide(), provider2.provide());

        // outputs are different for the same randomness and different key
        let mut provider1 = sub_randomness_with_key([0xA6; 32], "/my_namespace/ab");
        let mut provider2 = sub_randomness_with_key([0xA6; 32], "/my_namespace/cd");
        assert_ne!(provider1.provide(), provider2.provide());
        assert_ne!(provider1.provide(), provider2.provide());
        assert_ne!(provider1.provide(), provider2.provide());
    }

    #[test]
    fn sub_randomness_works() {
        let randomness: [u8; 32] = [0x77; 32];
        let mut provider = sub_randomness(randomness);
        let v1 = provider.provide();
        let v2 = provider.provide();
        let v3 = provider.provide();
        let v4 = provider.provide();
        println!("v1 = {v1:?}");
        println!("v2 = {v2:?}");
        println!("v3 = {v3:?}");
        println!("v4 = {v4:?}");

        // outputs are the same for the same randomness
        let mut provider1 = sub_randomness([0xA6; 32]);
        let mut provider2 = sub_randomness([0xA6; 32]);
        assert_eq!(provider1.provide(), provider2.provide());
        assert_eq!(provider1.provide(), provider2.provide());
        assert_eq!(provider1.provide(), provider2.provide());

        // outputs differ for different randomness
        let mut provider1 = sub_randomness([0xA6; 32]);
        let mut provider2 = sub_randomness([0xCF; 32]);
        assert_ne!(provider1.provide(), provider2.provide());
        assert_ne!(provider1.provide(), provider2.provide());
        assert_ne!(provider1.provide(), provider2.provide());

        // outputs are the same for the same as sub_randomness_with_key with "_^default^_"
        let mut provider1 = sub_randomness([0xA6; 32]);
        let mut provider2 = sub_randomness_with_key([0xA6; 32], "_^default^_");
        assert_eq!(provider1.provide(), provider2.provide());
        assert_eq!(provider1.provide(), provider2.provide());
        assert_eq!(provider1.provide(), provider2.provide());
    }

    #[test]
    fn sub_randomness_implements_iterator() {
        let randomness: [u8; 32] = [0x77; 32];
        let mut provider = sub_randomness(randomness);
        let v1 = provider.next().unwrap();
        let v2 = provider.next().unwrap();
        let v3 = provider.next().unwrap();
        let v4 = provider.next().unwrap();
        println!("v1 = {v1:?}");
        println!("v2 = {v2:?}");
        println!("v3 = {v3:?}");
        println!("v4 = {v4:?}");
    }
}
