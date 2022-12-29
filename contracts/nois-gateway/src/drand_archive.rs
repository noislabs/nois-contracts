use cosmwasm_std::{HexBinary, Storage};

pub fn archive_lookup(storage: &dyn Storage, round: u64) -> Option<HexBinary> {
    let key = drand_mainnet_randomness_key(round);
    storage.get(&key).map(Into::into)
}

pub fn archive_store(storage: &mut dyn Storage, round: u64, randomness: &HexBinary) {
    let key = drand_mainnet_randomness_key(round);
    storage.set(&key, randomness);
}

// Use raw storage key to allow storing and querying rounds
// without serde
fn drand_mainnet_randomness_key(round: u64) -> [u8; 11] {
    let bytes = round.to_be_bytes();
    [
        7,    // BELL
        b'd', // drand
        b'm', // mainnet
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]
}
