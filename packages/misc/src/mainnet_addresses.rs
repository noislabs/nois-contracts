#![allow(unused)]

use bech32::{FromBase32, ToBase32, Variant};
use cosmwasm_std::{instantiate2_address, CanonicalAddr};

const DEPLOYMENTS: &str = "nois1p9tw323xdjp5q3yzuecfahmgrpufmm89z93wpk";
const PREFIX: &str = "nois";
const SALT: &[u8] = b"\x01";

fn icecube_address() -> String {
    // See `curl -L https://github.com/noislabs/nois-contracts/releases/download/v0.11.0/checksums.txt | grep icecube`
    let checksum =
        hex::decode("b8cc8ae4201ee757a70c1e6d8b72c3afb94fc36ba95294ab0614011069de0fe5").unwrap();

    let (_, creator_data, variant) = bech32::decode(DEPLOYMENTS).unwrap();
    let creator: CanonicalAddr = Vec::<u8>::from_base32(&creator_data).unwrap().into();

    let icecube_data = instantiate2_address(&checksum, &creator, SALT).unwrap();

    bech32::encode(PREFIX, icecube_data.to_vec().to_base32(), Variant::Bech32).unwrap()
}

fn sink_address() -> String {
    // See `curl -L https://github.com/noislabs/nois-contracts/releases/download/v0.11.0/checksums.txt | grep sink`
    let checksum =
        hex::decode("cb7b021831dd908b673491288b7a76f7ae973b8c1586ce41d9a69cadab5202c7").unwrap();

    let (_, creator_data, variant) = bech32::decode(DEPLOYMENTS).unwrap();
    let creator: CanonicalAddr = Vec::<u8>::from_base32(&creator_data).unwrap().into();

    let sink_data = instantiate2_address(&checksum, &creator, SALT).unwrap();

    bech32::encode(PREFIX, sink_data.to_vec().to_base32(), Variant::Bech32).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icecube_address_works() {
        let addr = icecube_address();
        assert_eq!(
            addr,
            "nois1gwnfyx82rwgc4y9r8vx6nr9v35dwezw3dadw6h39mad9amg7shnsler5f0"
        );
    }

    #[test]
    fn sink_address_works() {
        let addr = sink_address();
        assert_eq!(
            addr,
            "nois10c0ppz0n57hqrmfp7g7lqs6k4xk9rxhvcfkqt83r8mars2lc57mq0f6cty"
        );
    }
}
