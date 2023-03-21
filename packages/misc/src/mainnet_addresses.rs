#![allow(unused)]

use bech32::{FromBase32, ToBase32, Variant};
use cosmwasm_std::{instantiate2_address, CanonicalAddr};

const DEPLOYMENTS: &str = "nois1p9tw323xdjp5q3yzuecfahmgrpufmm89z93wpk";
const PREFIX: &str = "nois";
const SALT: &[u8] = b"\x01";

fn icecube_address() -> String {
    // See `curl -L https://github.com/noislabs/nois-contracts/releases/download/v0.10.0/checksums.txt`
    let checksum =
        hex::decode("2856fea75fd9de874e85e835f8de2372532694f1b310a7ffe341be9e16ce183a").unwrap();

    let (_, creator_data, variant) = bech32::decode(DEPLOYMENTS).unwrap();
    let creator: CanonicalAddr = Vec::<u8>::from_base32(&creator_data).unwrap().into();

    let icecube_data = instantiate2_address(&checksum, &creator, SALT).unwrap();

    bech32::encode(PREFIX, icecube_data.to_vec().to_base32(), Variant::Bech32).unwrap()
}

fn sink_address() -> String {
    // See `curl -L https://github.com/noislabs/nois-contracts/releases/download/v0.10.0/checksums.txt`
    let checksum =
        hex::decode("6d0b1b81c2ae10b0772e22041ff179d82b67a50cc2dc9a645728666ef96a9682").unwrap();

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
            "nois1mdjzz7y7crnjgfkdld4x9ravqymh22jy2c7jpukqnkghhzv5s9ys2a9hu7"
        );
    }

    #[test]
    fn sink_address_works() {
        let addr = sink_address();
        assert_eq!(
            addr,
            "nois16rpxzry8jf06j8htjphf0lnmylh28enwm4xwkfpn3699wvyys2yqgcev3h"
        );
    }
}
