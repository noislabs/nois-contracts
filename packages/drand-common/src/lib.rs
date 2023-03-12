mod rounds;
pub mod testing;

pub use rounds::{time_of_round, valid_round_after};

use cosmwasm_std::Timestamp;

/// The chain hash serves as a drand network identifier.
///
/// See <https://drand.love/developer/> and <https://drand.cloudflare.com/info>
pub const DRAND_CHAIN_HASH: &str =
    "dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493";

// https://api3.drand.sh/dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493/info
pub const DRAND_GENESIS: Timestamp = Timestamp::from_seconds(1677685200);
pub const DRAND_ROUND_LENGTH: u64 = 3_000_000_000; // in nanoseconds
