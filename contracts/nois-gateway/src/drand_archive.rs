use cosmwasm_std::{HexBinary, Storage};
use drand_common::DrandNetwork::{self, *};

pub fn archive_lookup(
    storage: &dyn Storage,
    network: DrandNetwork,
    round: u64,
) -> Option<HexBinary> {
    let key = drand_randomness_key(network, round);
    storage.get(&key).map(Into::into)
}

pub fn archive_store(
    storage: &mut dyn Storage,
    network: DrandNetwork,
    round: u64,
    randomness: &HexBinary,
) {
    let key = drand_randomness_key(network, round);
    storage.set(&key, randomness);
}

// Use raw storage key to allow storing and querying rounds
// without serde
fn drand_randomness_key(network: DrandNetwork, round: u64) -> [u8; 11] {
    let network = match network {
        Fastnet => b'm',  // mainnet,
        Quicknet => b'q', // quicknet,
    };
    let round = round.to_be_bytes();
    [
        7,    // BELL
        b'd', // drand
        network, round[0], round[1], round[2], round[3], round[4], round[5], round[6], round[7],
    ]
}
