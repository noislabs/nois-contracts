mod rounds;
pub mod testing;

pub use rounds::{is_incentivized, round_after, time_of_round};

use cosmwasm_std::Timestamp;

/// The chain hash serves as a drand network identifier.
///
/// See <https://drand.love/developer/> and <https://drand.cloudflare.com/info>
pub const DRAND_CHAIN_HASH: &str =
    "52db9ba70e0cc0f6eaf7803dd07447a1f5477735fd3f661792ba94600c84e971";

// https://api3.drand.sh/52db9ba70e0cc0f6eaf7803dd07447a1f5477735fd3f661792ba94600c84e971/info
pub const DRAND_GENESIS: Timestamp = Timestamp::from_seconds(1692803367);
pub const DRAND_ROUND_LENGTH: u64 = 3_000_000_000; // in nanoseconds

/// The pubkey for fastnet (<https://api3.drand.sh/52db9ba70e0cc0f6eaf7803dd07447a1f5477735fd3f661792ba94600c84e971/info>)
pub const DRAND_MAINNET2_PUBKEY: [u8; 96] = [
    // $ node
    // > Uint8Array.from(Buffer.from("83cf0f2896adee7eb8b5f01fcad3912212c437e0073e911fb90022d3e760183c8c4b450b6a0a6c3ac6a5776a2d1064510d1fec758c921cc22b0e17e63aaf4bcb5ed66304de9cf809bd274ca73bab4af5a6e9c76a4bc09e76eae8991ef5ece45a", "hex"))
    131, 207, 15, 40, 150, 173, 238, 126, 184, 181, 240, 31, 202, 211, 145, 34, 18, 196, 55, 224, 7,
    62, 145, 31, 185, 0, 34, 211, 231, 96, 24, 60, 140, 75, 69, 11, 106, 10, 108, 58, 198, 165,
    119, 106, 45, 16, 100, 81, 13, 31, 236, 117, 140, 146, 28, 194, 43, 14, 23, 230, 58, 175, 75,
    203, 94, 214, 99, 4, 222, 156, 248, 9, 189, 39, 76, 167, 59, 171, 74, 245, 166, 233, 199, 106,
    75, 192, 158, 118, 234, 232, 153, 30, 245, 236, 228, 90,
];
