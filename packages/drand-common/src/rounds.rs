use cosmwasm_std::Timestamp;

use crate::{DRAND_GENESIS, DRAND_ROUND_LENGTH};

// See TimeOfRound implementation: https://github.com/drand/drand/blob/eb36ba81e3f28c966f95bcd602f60e7ff8ef4c35/chain/time.go#L30-L33
pub fn time_of_round(round: u64) -> Timestamp {
    DRAND_GENESIS.plus_nanos((round - 1) * DRAND_ROUND_LENGTH)
}

fn round_after(base: Timestamp) -> u64 {
    // Losely ported from https://github.com/drand/drand/blob/eb36ba81e3f28c966f95bcd602f60e7ff8ef4c35/chain/time.go#L49-L63
    if base < DRAND_GENESIS {
        1
    } else {
        let from_genesis = base.nanos() - DRAND_GENESIS.nanos();
        let periods_since_genesis = from_genesis / DRAND_ROUND_LENGTH;
        let next_period_index = periods_since_genesis + 1;
        next_period_index + 1 // Convert 0-based counting to 1-based counting
    }
}

/// Returns the next round after the timestamp which can be divided by the divisor.
fn round_after_divisor(base: Timestamp, divisor: u64) -> u64 {
    let round = round_after(base);
    let remainder = round % divisor;
    if remainder != 0 {
        round + divisor - remainder
    } else {
        round
    }
}

pub fn valid_round_after(base: Timestamp) -> u64 {
    round_after_divisor(base, 10)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_of_round_works() {
        assert_eq!(time_of_round(1), DRAND_GENESIS);
        assert_eq!(time_of_round(2), DRAND_GENESIS.plus_seconds(3));
        assert_eq!(time_of_round(111765), Timestamp::from_seconds(1678020492));
    }

    #[test]
    #[should_panic(expected = "overflow")]
    fn time_of_round_panics_for_round_0() {
        time_of_round(0);
    }

    #[test]
    fn round_after_works() {
        // UNIX epoch
        let round = round_after(Timestamp::from_seconds(0));
        assert_eq!(round, 1);

        // Before Drand genesis (https://api3.drand.sh/dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493/info)
        let round = round_after(Timestamp::from_seconds(1677685200).minus_nanos(1));
        assert_eq!(round, 1);

        // At Drand genesis
        let round = round_after(Timestamp::from_seconds(1677685200));
        assert_eq!(round, 2);

        // After Drand genesis
        let round = round_after(Timestamp::from_seconds(1677685200).plus_nanos(1));
        assert_eq!(round, 2);

        // Drand genesis +2s/3s/4s
        let round = round_after(Timestamp::from_seconds(1677685200).plus_seconds(2));
        assert_eq!(round, 2);
        let round = round_after(Timestamp::from_seconds(1677685200).plus_seconds(3));
        assert_eq!(round, 3);
        let round = round_after(Timestamp::from_seconds(1677685200).plus_seconds(4));
        assert_eq!(round, 3);
    }

    #[test]
    fn round_after_divisor_works() {
        // Before Drand genesis
        let before = Timestamp::from_seconds(1677685200).minus_seconds(1);
        assert_eq!(round_after_divisor(before, 1), 1);
        assert_eq!(round_after_divisor(before, 2), 2);
        assert_eq!(round_after_divisor(before, 10), 10);
        assert_eq!(round_after_divisor(before, 700), 700);

        // At Drand genesis
        let genesis = Timestamp::from_seconds(1677685200);
        assert_eq!(round_after_divisor(genesis, 1), 2);
        assert_eq!(round_after_divisor(genesis, 2), 2);
        assert_eq!(round_after_divisor(genesis, 10), 10);
        assert_eq!(round_after_divisor(genesis, 700), 700);

        let after5 = genesis.plus_seconds(5);
        assert_eq!(round_after_divisor(after5, 1), 3);
        assert_eq!(round_after_divisor(after5, 3), 3);
        assert_eq!(round_after_divisor(after5, 10), 10);

        let later = genesis.plus_seconds(299);
        assert_eq!(round_after_divisor(later, 1), 101);
        assert_eq!(round_after_divisor(later, 10), 110);
    }
}
