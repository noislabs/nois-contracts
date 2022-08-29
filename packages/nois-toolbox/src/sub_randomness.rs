use rand_chacha::{rand_core::RngCore, ChaCha8Rng};

use crate::prng::make_prng;

pub struct SubRandomnessProvider {
    rng: ChaCha8Rng,
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

/// Takes a randomness and returns an arbitrary number of sub-randomnesses.
pub fn sub_randomness(randomness: [u8; 32]) -> Box<SubRandomnessProvider> {
    let rng = make_prng(randomness);

    Box::new(SubRandomnessProvider { rng })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sub_randomness_works() {
        let randomness: [u8; 32] = [0x77; 32];
        let mut generator = sub_randomness(randomness);
        let v1 = generator.next().unwrap();
        let v2 = generator.next().unwrap();
        let v3 = generator.next().unwrap();
        let v4 = generator.next().unwrap();
        println!("v1 = {v1:?}");
        println!("v2 = {v2:?}");
        println!("v3 = {v3:?}");
        println!("v4 = {v4:?}");
    }
}
