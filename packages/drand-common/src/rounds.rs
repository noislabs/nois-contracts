use cosmwasm_std::Timestamp;

use crate::{DRAND_GENESIS, DRAND_ROUND_LENGTH};

// See TimeOfRound implementation: https://github.com/drand/drand/blob/eb36ba81e3f28c966f95bcd602f60e7ff8ef4c35/chain/time.go#L30-L33
pub fn time_of_round(round: u64) -> Timestamp {
    DRAND_GENESIS.plus_nanos((round - 1) * DRAND_ROUND_LENGTH)
}

pub fn round_after(base: Timestamp) -> u64 {
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

/// Returns true if and only if the round number is incentivized for Nois.
///
/// - For mainnet launch (< 0.15.0), every 10th round is considered valid.
/// - For fast randomness (>= 0.15.0), all rounds are valid but only every 10th round is incentivized.
/// - For fast randomness (>= 0.15.1), all rounds are valid but only every 15th round is incentivized.
/// - For fast randomness (>= 0.15.3), all rounds are valid but only every 25th round is incentivized.
///
/// If round is 0, this returns false because there is no 0 round in drand.
#[inline]
pub fn is_incentivized(round: u64) -> bool {
    round != 0 && round % 25 == 0
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
    fn is_incentivised_works() {
        assert!(!is_incentivized(0)); // no 0 round exists in drand
        assert!(!is_incentivized(1));
        assert!(!is_incentivized(2));
        assert!(!is_incentivized(3));
        assert!(!is_incentivized(4));
        assert!(!is_incentivized(5));
        assert!(!is_incentivized(6));
        assert!(!is_incentivized(7));
        assert!(!is_incentivized(8));
        assert!(!is_incentivized(9));
        assert!(!is_incentivized(10));
        assert!(!is_incentivized(11));
        assert!(!is_incentivized(12));
        assert!(!is_incentivized(13));
        assert!(!is_incentivized(14));
        assert!(!is_incentivized(15));
        assert!(!is_incentivized(16));
        assert!(!is_incentivized(17));
        assert!(!is_incentivized(18));
        assert!(!is_incentivized(19));
        assert!(!is_incentivized(20));
        assert!(!is_incentivized(21));
        assert!(!is_incentivized(22));
        assert!(!is_incentivized(23));
        assert!(!is_incentivized(24));
        assert!(is_incentivized(25));
        assert!(!is_incentivized(26));
        assert!(!is_incentivized(27));
        assert!(!is_incentivized(28));
        assert!(!is_incentivized(29));
        assert!(!is_incentivized(30));
        assert!(!is_incentivized(31));
        assert!(!is_incentivized(32));
        assert!(!is_incentivized(33));
        assert!(!is_incentivized(34));
        assert!(!is_incentivized(35));
        assert!(!is_incentivized(36));
        assert!(!is_incentivized(37));
        assert!(!is_incentivized(38));
        assert!(!is_incentivized(39));
        assert!(!is_incentivized(40));
        assert!(!is_incentivized(41));
        assert!(!is_incentivized(42));
        assert!(!is_incentivized(43));
        assert!(!is_incentivized(44));
        assert!(!is_incentivized(45));
        assert!(!is_incentivized(46));
        assert!(!is_incentivized(47));
        assert!(!is_incentivized(48));
        assert!(!is_incentivized(49));
        assert!(is_incentivized(50));
        assert!(!is_incentivized(51));
    }
}
