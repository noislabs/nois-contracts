use cosmwasm_std::Timestamp;

/// The chain hash serves as a drand network identifier.
///
/// See <https://drand.love/developer/> and <https://drand.cloudflare.com/info>
pub const DRAND_CHAIN_HASH: &str =
    "8990e7a9aaed2ffed73dbd7092123d6f289930540d7651336225dc172e51b2ce";

pub const DRAND_GENESIS: Timestamp = Timestamp::from_seconds(1595431050);
pub const DRAND_ROUND_LENGTH: u64 = 30_000_000_000; // in nanoseconds

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
