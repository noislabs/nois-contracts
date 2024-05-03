mod rounds;
pub mod testing;

use core::fmt;

pub use rounds::{is_incentivized, round_after, time_of_round};

use cosmwasm_std::Timestamp;

#[derive(Debug, Clone, Copy)]
pub enum DrandNetwork {
    Fastnet,
    Quicknet,
}

impl DrandNetwork {
    pub fn genesis_time(&self) -> Timestamp {
        match self {
            // https://api3.drand.sh/dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493/info
            DrandNetwork::Fastnet => Timestamp::from_seconds(1677685200),
            DrandNetwork::Quicknet => Timestamp::from_seconds(1692803367),
        }
    }

    pub fn chain_hash(&self) -> &'static str {
        match self {
            DrandNetwork::Fastnet => {
                "dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493"
            }
            // https://drand.love/blog/2023/10/16/quicknet-is-live/
            DrandNetwork::Quicknet => {
                "52db9ba70e0cc0f6eaf7803dd07447a1f5477735fd3f661792ba94600c84e971"
            }
        }
    }

    pub const fn pubkey(&self) -> [u8; 96] {
        match self {
            // The pubkey for fastnet (<https://api3.drand.sh/dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493/info>)
            DrandNetwork::Fastnet => [
                // $ node
                // > Uint8Array.from(Buffer.from("a0b862a7527fee3a731bcb59280ab6abd62d5c0b6ea03dc4ddf6612fdfc9d01f01c31542541771903475eb1ec6615f8d0df0b8b6dce385811d6dcf8cbefb8759e5e616a3dfd054c928940766d9a5b9db91e3b697e5d70a975181e007f87fca5e", "hex"))
                160, 184, 98, 167, 82, 127, 238, 58, 115, 27, 203, 89, 40, 10, 182, 171, 214, 45,
                92, 11, 110, 160, 61, 196, 221, 246, 97, 47, 223, 201, 208, 31, 1, 195, 21, 66, 84,
                23, 113, 144, 52, 117, 235, 30, 198, 97, 95, 141, 13, 240, 184, 182, 220, 227, 133,
                129, 29, 109, 207, 140, 190, 251, 135, 89, 229, 230, 22, 163, 223, 208, 84, 201,
                40, 148, 7, 102, 217, 165, 185, 219, 145, 227, 182, 151, 229, 215, 10, 151, 81,
                129, 224, 7, 248, 127, 202, 94,
            ],
            // The pubkey for quicknet (<https://api3.drand.sh/52db9ba70e0cc0f6eaf7803dd07447a1f5477735fd3f661792ba94600c84e971/info>)
            DrandNetwork::Quicknet => [
                // $ node
                // > Uint8Array.from(Buffer.from("83cf0f2896adee7eb8b5f01fcad3912212c437e0073e911fb90022d3e760183c8c4b450b6a0a6c3ac6a5776a2d1064510d1fec758c921cc22b0e17e63aaf4bcb5ed66304de9cf809bd274ca73bab4af5a6e9c76a4bc09e76eae8991ef5ece45a", "hex"))
                131, 207, 15, 40, 150, 173, 238, 126, 184, 181, 240, 31, 202, 211, 145, 34, 18, 196,
                55, 224, 7, 62, 145, 31, 185, 0, 34, 211, 231, 96, 24, 60, 140, 75, 69, 11, 106,
                10, 108, 58, 198, 165, 119, 106, 45, 16, 100, 81, 13, 31, 236, 117, 140, 146, 28,
                194, 43, 14, 23, 230, 58, 175, 75, 203, 94, 214, 99, 4, 222, 156, 248, 9, 189, 39,
                76, 167, 59, 171, 74, 245, 166, 233, 199, 106, 75, 192, 158, 118, 234, 232, 153,
                30, 245, 236, 228, 90,
            ],
        }
    }
}

impl fmt::Display for DrandNetwork {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DrandNetwork::Fastnet => f.write_str("fastnet"),
            DrandNetwork::Quicknet => f.write_str("quicknet"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_string_works() {
        assert_eq!(DrandNetwork::Fastnet.to_string(), "fastnet");
        assert_eq!(DrandNetwork::Quicknet.to_string(), "quicknet");
    }
}
