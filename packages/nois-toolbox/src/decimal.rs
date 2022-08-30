use cosmwasm_std::Decimal;
use rand_xoshiro::rand_core::RngCore;

use crate::prng::make_prng;

const fn u128_from_parts(high: u64, low: u64) -> u128 {
    let high = high.to_be_bytes();
    let low = low.to_be_bytes();
    u128::from_be_bytes([
        high[0], high[1], high[2], high[3], high[4], high[5], high[6], high[7], low[0], low[1],
        low[2], low[3], low[4], low[5], low[6], low[7],
    ])
}

/// Returns a Decimal d with 0 <= d < 1
pub fn random_decimal(randomness: [u8; 32]) -> Decimal {
    let mut rng = make_prng(randomness);
    let mut value = u128_from_parts(rng.next_u64(), rng.next_u64());
    // Using mod to get a random value in [0, 10**18) should be alright
    // since 10**18 is small compared to 2**128-1
    value %= 1000000000000000000;

    // Cannot overflow since value < 10**18 and we use the native decimal places of
    // the Decimal type.
    Decimal::from_atomics(value, 18).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::str::FromStr;

    #[test]
    fn u128_from_parts_works() {
        assert_eq!(u128_from_parts(0, 0), 0);
        assert_eq!(u128_from_parts(0, 1), 1);
        assert_eq!(u128_from_parts(1, 0), 1 << 64);
        assert_eq!(
            u128_from_parts(0xA123456789ABCDEF, 0xD2E3F4A6C7992242),
            0xA123456789ABCDEFD2E3F4A6C7992242
        );
    }

    #[test]
    fn random_decimal_works() {
        let randomnesses: [[u8; 32]; 8] = [
            [0x00; 32], [0x01; 32], [0x20; 32], [0x23; 32], [0x4A; 32], [0xBB; 32], [0xDE; 32],
            [0xFF; 32],
        ];

        for randomness in randomnesses {
            let a = random_decimal(randomness);
            println!("{a}");
            assert!(a < Decimal::one());
        }

        const ROUNDS: u32 = 50_000;
        const MAGIC_BYTE: u8 = 0xA8; // use to change randomness in tests manually
        let mut values = Vec::<Decimal>::with_capacity(ROUNDS as usize);
        for i in 0..ROUNDS {
            let i_bytes = i.to_be_bytes();
            let randomness: [u8; 32] = [
                MAGIC_BYTE, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, i_bytes[0], i_bytes[1], i_bytes[2], i_bytes[3],
            ];

            let a = random_decimal(randomness);
            values.push(a);
        }

        let d0_0 = Decimal::from_str("0.0").unwrap();
        let d0_1 = Decimal::from_str("0.1").unwrap();
        let d0_2 = Decimal::from_str("0.2").unwrap();
        let d0_3 = Decimal::from_str("0.3").unwrap();
        let d0_4 = Decimal::from_str("0.4").unwrap();
        let d0_5 = Decimal::from_str("0.5").unwrap();
        let d0_6 = Decimal::from_str("0.6").unwrap();
        let d0_7 = Decimal::from_str("0.7").unwrap();
        let d0_8 = Decimal::from_str("0.8").unwrap();
        let d0_9 = Decimal::from_str("0.9").unwrap();
        let d1_0 = Decimal::from_str("1.0").unwrap();

        let c1 = values.iter().filter(|x| **x >= d0_0 && **x < d0_1).count();
        let c2 = values.iter().filter(|x| **x >= d0_1 && **x < d0_2).count();
        let c3 = values.iter().filter(|x| **x >= d0_2 && **x < d0_3).count();
        let c4 = values.iter().filter(|x| **x >= d0_3 && **x < d0_4).count();
        let c5 = values.iter().filter(|x| **x >= d0_4 && **x < d0_5).count();
        let c6 = values.iter().filter(|x| **x >= d0_5 && **x < d0_6).count();
        let c7 = values.iter().filter(|x| **x >= d0_6 && **x < d0_7).count();
        let c8 = values.iter().filter(|x| **x >= d0_7 && **x < d0_8).count();
        let c9 = values.iter().filter(|x| **x >= d0_8 && **x < d0_9).count();
        let c10 = values.iter().filter(|x| **x >= d0_9 && **x < d1_0).count();

        println!("Distribution: {c1}, {c2}, {c3}, {c4}, {c5}, {c6}, {c7}, {c8}, {c9}, {c10}");

        assert!((4_800..=5_200).contains(&c1));
        assert!((4_800..=5_200).contains(&c2));
        assert!((4_800..=5_200).contains(&c3));
        assert!((4_800..=5_200).contains(&c4));
        assert!((4_800..=5_200).contains(&c5));
        assert!((4_800..=5_200).contains(&c6));
        assert!((4_800..=5_200).contains(&c7));
        assert!((4_800..=5_200).contains(&c8));
        assert!((4_800..=5_200).contains(&c9));
        assert!((4_800..=5_200).contains(&c10));

        assert_eq!(
            c1 + c2 + c3 + c4 + c5 + c6 + c7 + c8 + c9 + c10,
            ROUNDS as usize
        );
    }
}
