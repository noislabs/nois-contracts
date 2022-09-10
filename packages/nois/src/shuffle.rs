use rand::Rng;

use crate::prng::make_prng;

/// Shuffles a vector using the Fisher-Yates algorithm
pub fn shuffle<T>(randomness: [u8; 32], data: &mut Vec<T>) {
    let mut rng = make_prng(randomness);
    for i in (1..data.len()).rev() {
        let j = rng.gen_range(0..=i);
        data.swap(i, j);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RANDOMNESS1: [u8; 32] = [
        52, 187, 72, 255, 102, 110, 115, 233, 50, 165, 124, 255, 217, 131, 112, 209, 253, 176, 108,
        99, 102, 225, 12, 36, 82, 107, 106, 207, 99, 107, 197, 84,
    ];

    #[test]
    fn shuffle_works() {
        let mut data: Vec<i32> = vec![];
        shuffle(RANDOMNESS1, &mut data);
        assert_eq!(data, Vec::<i32>::new());

        let mut data = vec![5];
        shuffle(RANDOMNESS1, &mut data);
        assert_eq!(data, vec![5]);

        // Order has changed for larger vector
        let mut data = vec![1, 2, 3, 4];
        shuffle(RANDOMNESS1, &mut data);
        assert_eq!(data.len(), 4);
        assert_ne!(data, vec![1, 2, 3, 4]);
    }
}
