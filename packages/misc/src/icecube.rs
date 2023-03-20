#![allow(unused)]

use bech32::{FromBase32, ToBase32, Variant};
use cosmwasm_std::{instantiate2_address, CanonicalAddr};

fn icecube_address() -> String {
    // See `curl -L https://github.com/noislabs/nois-contracts/releases/download/v0.10.0/checksums.txt`
    let checksum =
        hex::decode("2856fea75fd9de874e85e835f8de2372532694f1b310a7ffe341be9e16ce183a").unwrap();

    let (_, creator_data, variant) =
        bech32::decode("nois1p9tw323xdjp5q3yzuecfahmgrpufmm89z93wpk").unwrap();
    let creator: CanonicalAddr = Vec::<u8>::from_base32(&creator_data).unwrap().into();

    let salt = [0x01];
    let icecube_data = instantiate2_address(&checksum, &creator, &salt).unwrap();

    bech32::encode("nois", icecube_data.to_vec().to_base32(), Variant::Bech32).unwrap()
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
}
