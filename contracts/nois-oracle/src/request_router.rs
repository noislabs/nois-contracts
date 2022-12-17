//! The request router module decides which randomness backend is used

use cosmwasm_std::Timestamp;

use crate::drand::{round_after, DRAND_CHAIN_HASH};

/// Calculates the next round in the future, i.e. publish time > base time.
pub fn commit_to_drand_round(after: Timestamp) -> (u64, String) {
    let round = round_after(after);
    let source_id = format!("drand:{}:{}", DRAND_CHAIN_HASH, round);
    (round, source_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commit_to_drand_round_works() {
        // UNIX epoch
        let (round, source) = commit_to_drand_round(Timestamp::from_seconds(0));
        assert_eq!(round, 1);
        assert_eq!(
            source,
            "drand:8990e7a9aaed2ffed73dbd7092123d6f289930540d7651336225dc172e51b2ce:1"
        );

        // Before Drand genesis (https://api3.drand.sh/info)
        let (round, source) =
            commit_to_drand_round(Timestamp::from_seconds(1595431050).minus_nanos(1));
        assert_eq!(round, 1);
        assert_eq!(
            source,
            "drand:8990e7a9aaed2ffed73dbd7092123d6f289930540d7651336225dc172e51b2ce:1"
        );

        // At Drand genesis (https://api3.drand.sh/info)
        let (round, source) = commit_to_drand_round(Timestamp::from_seconds(1595431050));
        assert_eq!(round, 2);
        assert_eq!(
            source,
            "drand:8990e7a9aaed2ffed73dbd7092123d6f289930540d7651336225dc172e51b2ce:2"
        );

        // After Drand genesis (https://api3.drand.sh/info)
        let (round, _) = commit_to_drand_round(Timestamp::from_seconds(1595431050).plus_nanos(1));
        assert_eq!(round, 2);

        // Drand genesis +29s/30s/31s
        let (round, _) =
            commit_to_drand_round(Timestamp::from_seconds(1595431050).plus_seconds(29));
        assert_eq!(round, 2);
        let (round, _) =
            commit_to_drand_round(Timestamp::from_seconds(1595431050).plus_seconds(30));
        assert_eq!(round, 3);
        let (round, _) =
            commit_to_drand_round(Timestamp::from_seconds(1595431050).plus_seconds(31));
        assert_eq!(round, 3);
    }
}
