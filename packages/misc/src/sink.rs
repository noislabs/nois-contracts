#![allow(unused)]

use bech32::{FromBase32, ToBase32, Variant};
use cosmwasm_std::{instantiate2_address, CanonicalAddr};

fn sink_address() -> String {
    // See `curl -L https://github.com/noislabs/nois-contracts/releases/download/v0.10.0/checksums.txt`
    let checksum =
        hex::decode("6d0b1b81c2ae10b0772e22041ff179d82b67a50cc2dc9a645728666ef96a9682").unwrap();

    let (_, creator_data, variant) =
        bech32::decode("nois1p9tw323xdjp5q3yzuecfahmgrpufmm89z93wpk").unwrap();
    let creator: CanonicalAddr = Vec::<u8>::from_base32(&creator_data).unwrap().into();

    let salt = [0x01];
    let sink_data = instantiate2_address(&checksum, &creator, &salt).unwrap();

    bech32::encode("nois", sink_data.to_vec().to_base32(), Variant::Bech32).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sink_address_works() {
        let addr = sink_address();
        assert_eq!(
            addr,
            "nois16rpxzry8jf06j8htjphf0lnmylh28enwm4xwkfpn3699wvyys2yqgcev3h"
        );
    }
}
