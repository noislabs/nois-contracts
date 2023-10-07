use cosmwasm_std::Addr;
use sha2::{Digest, Sha256};

use crate::error::ContractError;

const MAX_MONIKER_LEN: usize = 20;

pub fn validate_moniker(moniker: &str) -> Result<(), ContractError> {
    if moniker.is_empty() {
        Err(ContractError::MonikerEmpty)
    } else if moniker.len() > MAX_MONIKER_LEN {
        Err(ContractError::MonikerTooLong)
    } else {
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Group {
    /// Byte is 0 modulo 2 (even)
    A,
    /// Byte is 1 modulo 2 (odd)
    B,
}

pub fn group(addr: &Addr) -> Group {
    let mut hasher = Sha256::new();
    hasher.update(addr.as_bytes());
    let hash: [u8; 32] = hasher.finalize().into();
    if hash[0] % 2 == 0 {
        Group::A
    } else {
        Group::B
    }
}

/// The groups which is eligable to get rewards for this round.
///
/// Right now, even rounds are for group A and odd rounds are for group B.
pub fn eligible_group(round: u64) -> Group {
    if round % 2 == 0 {
        Group::A
    } else {
        Group::B
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_moniker_works() {
        validate_moniker("abc").unwrap();
        validate_moniker("stargaze.zone").unwrap();
        validate_moniker("webmaster128").unwrap();

        let err = validate_moniker("").unwrap_err();
        assert!(matches!(err, ContractError::MonikerEmpty));

        let err = validate_moniker("abcabcabcabcabcaabc2bcabcabcabcabcabcabc").unwrap_err();
        assert!(matches!(err, ContractError::MonikerTooLong));
    }

    #[test]
    fn group_works() {
        // Take the first byte of the hash and check if it is even or odd
        // echo -n nois1ffy2rz96sjxzm2ezwkmvyeupktp7elt6w3xckt | sha256sum | head -c 2
        // python3 -c 'print(0xc1 % 2)'
        // 1
        let addr = Addr::unchecked("nois1ffy2rz96sjxzm2ezwkmvyeupktp7elt6w3xckt");
        assert_eq!(group(&addr), Group::B);
        let addr = Addr::unchecked("nois1uw8c69maprjq5ure7x80x9nauasrn7why5dfwd");
        assert_eq!(group(&addr), Group::B);
        let addr = Addr::unchecked("nois1zh77twxfc47eu59q7mc7027jvvcnrpte3sr922");
        assert_eq!(group(&addr), Group::B);
        let addr = Addr::unchecked("nois1wpy3gwlw4tt3uy0u5jrspfz0w9azztvlr0d04s");
        assert_eq!(group(&addr), Group::A);
        let addr = Addr::unchecked("nois1rw47dxvhw3ahdlcznvwpcz43cdq8l0832eg6re");
        assert_eq!(group(&addr), Group::A);
        let addr = Addr::unchecked("nois12a8yv4ndgnkygujj7cmmkfz2j9wjanezldwye0");
        assert_eq!(group(&addr), Group::B);
    }

    #[test]
    fn eligible_group_works() {
        eligible_group(0); // undefined, not a valid drand round
        assert_eq!(eligible_group(1), Group::B);
        assert_eq!(eligible_group(2), Group::A);
        assert_eq!(eligible_group(3), Group::B);
        assert_eq!(eligible_group(4), Group::A);
        assert_eq!(eligible_group(5), Group::B);
        assert_eq!(eligible_group(6), Group::A);
    }
}
