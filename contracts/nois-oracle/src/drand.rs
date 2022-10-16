use cosmwasm_std::Timestamp;

// $ node
// > Uint8Array.from(Buffer.from("868f005eb8e6e4ca0a47c8a77ceaa5309a47978a7c71bc5cce96366b5d7a569937c529eeda66c7293784a9402801af31", "hex"))
pub const DRAND_MAINNET_PUBKEY: [u8; 48] = [
    134, 143, 0, 94, 184, 230, 228, 202, 10, 71, 200, 167, 124, 234, 165, 48, 154, 71, 151, 138,
    124, 113, 188, 92, 206, 150, 54, 107, 93, 122, 86, 153, 55, 197, 41, 238, 218, 102, 199, 41,
    55, 132, 169, 64, 40, 1, 175, 49,
];
pub const DRAND_CHAIN_HASH: &str =
    "8990e7a9aaed2ffed73dbd7092123d6f289930540d7651336225dc172e51b2ce"; // See https://drand.love/developer/
pub const DRAND_GENESIS: Timestamp = Timestamp::from_seconds(1595431050);
pub const DRAND_ROUND_LENGTH: u64 = 30_000_000_000; // in nanoseconds

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_of_round_works() {
        assert_eq!(time_of_round(1), DRAND_GENESIS);
        assert_eq!(time_of_round(2), DRAND_GENESIS.plus_seconds(30));
        assert_eq!(time_of_round(2238596), Timestamp::from_seconds(1662588900));
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

        // Before Drand genesis (https://api3.drand.sh/info)
        let round = round_after(Timestamp::from_seconds(1595431050).minus_nanos(1));
        assert_eq!(round, 1);

        // At Drand genesis (https://api3.drand.sh/info)
        let round = round_after(Timestamp::from_seconds(1595431050));
        assert_eq!(round, 2);

        // After Drand genesis (https://api3.drand.sh/info)
        let round = round_after(Timestamp::from_seconds(1595431050).plus_nanos(1));
        assert_eq!(round, 2);

        // Drand genesis +29s/30s/31s
        let round = round_after(Timestamp::from_seconds(1595431050).plus_seconds(29));
        assert_eq!(round, 2);
        let round = round_after(Timestamp::from_seconds(1595431050).plus_seconds(30));
        assert_eq!(round, 3);
        let round = round_after(Timestamp::from_seconds(1595431050).plus_seconds(31));
        assert_eq!(round, 3);
    }
}
