use rand::{
    distributions::{
        uniform::{SampleRange, SampleUniform},
        Distribution, Uniform,
    },
    Rng,
};

use crate::prng::make_prng;

/// Derives a random integer in the given range.
/// Use this method to avoid a modulo bias.
///
/// ## Example
///
/// ```
/// use nois::int_in_range;
///
/// # let randomness: [u8; 32] = [0x77; 32];
///
/// // Half-open interval [1, 7)
/// let dice1 = int_in_range(randomness, 1..7);
/// assert!(dice1 >= 1);
/// assert!(dice1 < 7);
///
/// // Closed interval [1, 6]
/// let dice2 = int_in_range(randomness, 1..=6);
/// assert!(dice2 >= 1);
/// assert!(dice2 <= 6);
/// ```
pub fn int_in_range<T, R>(randomness: [u8; 32], range: R) -> T
where
    T: SampleUniform + Int,
    R: SampleRange<T>,
{
    let mut rng = make_prng(randomness);
    rng.gen_range(range)
}

/// Derives random integers in the given range.
/// Use this method to avoid a modulo bias.
/// Using this is potentially more efficient than multiple calls of [`int_in_range`].
///
/// ## Example
///
/// ```
/// use nois::ints_in_range;
///
/// # let randomness: [u8; 32] = [0x77; 32];
///
/// let [dice1, dice2] = ints_in_range(randomness, 1..=6);
/// assert!(dice1 >= 1 && dice1 <= 6);
/// assert!(dice2 >= 1 && dice2 <= 6);
/// ```
pub fn ints_in_range<T, const LENGTH: usize, R>(randomness: [u8; 32], range: R) -> [T; LENGTH]
where
    T: SampleUniform + Int,
    R: Into<Uniform<T>>,
{
    let mut rng = make_prng(randomness);
    let uniform: Uniform<T> = range.into();
    let mut out = [T::default(); LENGTH];
    for o in out.iter_mut() {
        *o = uniform.sample(&mut rng);
    }
    out
}

/// A trait to restrict int types for [`int_in_range`]
pub trait Int: PartialOrd + Default + Copy {}

impl Int for u8 {}
impl Int for u16 {}
impl Int for u32 {}
impl Int for u64 {}
impl Int for u128 {}
impl Int for usize {}
impl Int for i8 {}
impl Int for i16 {}
impl Int for i32 {}
impl Int for i64 {}
impl Int for i128 {}
impl Int for isize {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn int_in_range_works() {
        // Half-open interval
        let result = int_in_range(
            [
                88, 85, 86, 91, 61, 64, 60, 71, 234, 24, 246, 200, 35, 73, 38, 187, 54, 59, 96, 9,
                237, 27, 215, 103, 148, 230, 28, 48, 51, 114, 203, 219,
            ],
            4..19,
        );
        assert_eq!(result, 11);

        let result = int_in_range(
            [
                207, 251, 10, 105, 100, 223, 244, 6, 207, 231, 253, 206, 157, 68, 143, 184, 209,
                222, 70, 249, 114, 160, 213, 73, 147, 94, 136, 191, 94, 98, 99, 170,
            ],
            4..19,
        );
        assert_eq!(result, 17);

        let result = int_in_range(
            [
                43, 140, 160, 0, 187, 41, 212, 6, 218, 53, 58, 198, 80, 209, 171, 239, 222, 247,
                30, 23, 184, 79, 79, 221, 192, 225, 217, 142, 135, 164, 169, 255,
            ],
            4..19,
        );
        assert_eq!(result, 6);

        let result = int_in_range(
            [
                43, 140, 160, 0, 187, 41, 212, 6, 218, 53, 58, 198, 80, 209, 171, 239, 222, 247,
                30, 23, 184, 79, 79, 221, 192, 225, 217, 142, 135, 164, 169, 255,
            ],
            123..124,
        );
        assert_eq!(result, 123);

        // Closed
        let result = int_in_range(
            [
                88, 85, 86, 91, 61, 64, 60, 71, 234, 24, 246, 200, 35, 73, 38, 187, 54, 59, 96, 9,
                237, 27, 215, 103, 148, 230, 28, 48, 51, 114, 203, 219,
            ],
            4..=19,
        );
        assert_eq!(result, 4);
        let result = int_in_range(
            [
                88, 85, 86, 91, 61, 64, 60, 71, 234, 24, 246, 200, 35, 73, 38, 187, 54, 59, 96, 9,
                237, 27, 215, 103, 148, 230, 28, 48, 51, 114, 203, 219,
            ],
            3..=3,
        );
        assert_eq!(result, 3);

        // Negative numbers
        let result = int_in_range(
            [
                74, 71, 86, 169, 247, 21, 60, 71, 234, 24, 246, 215, 35, 73, 38, 187, 54, 59, 96,
                9, 237, 27, 215, 103, 14, 230, 28, 48, 51, 114, 203, 219,
            ],
            -100..=100,
        );
        assert_eq!(result, -28);

        // u128
        let result = int_in_range(
            [
                74, 71, 86, 169, 247, 21, 60, 71, 234, 24, 246, 215, 35, 73, 38, 187, 54, 59, 96,
                9, 237, 27, 215, 103, 14, 230, 28, 48, 51, 114, 203, 219,
            ],
            17u128..=u128::MAX,
        );
        assert_eq!(result, 226364637901700064573816523306429827859);
    }

    #[test]
    #[should_panic = "cannot sample empty range"]
    fn int_in_range_panicks_for_empty() {
        let result = int_in_range(
            [
                52, 187, 72, 255, 102, 110, 115, 233, 50, 165, 124, 255, 217, 131, 112, 209, 253,
                176, 108, 99, 102, 225, 12, 36, 82, 107, 106, 207, 99, 107, 197, 84,
            ],
            4..4,
        );
        assert_eq!(result, 5);
    }

    #[test]
    fn ints_in_range_works() {
        // One output
        let result = ints_in_range(
            [
                88, 85, 86, 91, 61, 64, 60, 71, 234, 24, 246, 200, 35, 73, 38, 187, 54, 59, 96, 9,
                237, 27, 215, 103, 148, 230, 28, 48, 51, 114, 203, 219,
            ],
            4..19,
        );
        assert_eq!(result, [11]);

        // Two outputs
        let result = ints_in_range(
            [
                88, 85, 86, 91, 61, 64, 60, 71, 234, 24, 246, 200, 35, 73, 38, 187, 54, 59, 96, 9,
                237, 27, 215, 103, 148, 230, 28, 48, 51, 114, 203, 219,
            ],
            4..19,
        );
        assert_eq!(result, [11, 16]);
    }
}
