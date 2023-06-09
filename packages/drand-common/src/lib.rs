mod rounds;
pub mod testing;

pub use rounds::{is_valid, round_after, time_of_round};

use cosmwasm_std::Timestamp;

/// The chain hash serves as a drand network identifier.
///
/// See <https://drand.love/developer/> and <https://drand.cloudflare.com/info>
pub const DRAND_CHAIN_HASH: &str =
    "dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493";

// https://api3.drand.sh/dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493/info
pub const DRAND_GENESIS: Timestamp = Timestamp::from_seconds(1677685200);
pub const DRAND_ROUND_LENGTH: u64 = 3_000_000_000; // in nanoseconds

// $ node
// > Uint8Array.from(Buffer.from("a0b862a7527fee3a731bcb59280ab6abd62d5c0b6ea03dc4ddf6612fdfc9d01f01c31542541771903475eb1ec6615f8d0df0b8b6dce385811d6dcf8cbefb8759e5e616a3dfd054c928940766d9a5b9db91e3b697e5d70a975181e007f87fca5e", "hex"))
pub const DRAND_MAINNET2_PUBKEY: [u8; 96] = [
    160, 184, 98, 167, 82, 127, 238, 58, 115, 27, 203, 89, 40, 10, 182, 171, 214, 45, 92, 11, 110,
    160, 61, 196, 221, 246, 97, 47, 223, 201, 208, 31, 1, 195, 21, 66, 84, 23, 113, 144, 52, 117,
    235, 30, 198, 97, 95, 141, 13, 240, 184, 182, 220, 227, 133, 129, 29, 109, 207, 140, 190, 251,
    135, 89, 229, 230, 22, 163, 223, 208, 84, 201, 40, 148, 7, 102, 217, 165, 185, 219, 145, 227,
    182, 151, 229, 215, 10, 151, 81, 129, 224, 7, 248, 127, 202, 94,
];
