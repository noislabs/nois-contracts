use cosmwasm_std::Addr;
use drand_common::DRAND_ROUND_DIVISOR;
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
/// - 110147 skip
/// - 110148 skip
/// - 110149 skip
/// - 110150 A
/// - 110151 skip
/// - 110152 skip
/// - 110153 skip
/// - 110154 skip
/// - 110155 skip
/// - 110156 skip
/// - 110157 skip
/// - 110158 skip
/// - 110159 skip
/// - 110160 B
/// - 110161 skip
/// - 110162 skip
/// - 110163 skip
/// - 110164 skip
/// - 110165 skip
/// - 110166 skip
/// - 110167 skip
/// - 110168 skip
/// - 110169 skip
/// - 110170 A
/// - 110171 skip
/// - …
pub fn eligible_group(round: u64) -> Option<Group> {
    if round == 0 {
        return None;
    }

    if round % DRAND_ROUND_DIVISOR != 0 {
        return None;
    }

    if round % (DRAND_ROUND_DIVISOR * 2) == 0 {
        Some(Group::B)
    } else {
        Some(Group::A)
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
        assert_eq!(eligible_group(0), None);
        assert_eq!(eligible_group(1), None);
        assert_eq!(eligible_group(2), None);
        assert_eq!(eligible_group(3), Some(Group::A));
        assert_eq!(eligible_group(4), None);
        assert_eq!(eligible_group(5), None);
        assert_eq!(eligible_group(6), Some(Group::B));
        assert_eq!(eligible_group(7), None);
        assert_eq!(eligible_group(8), None);
        assert_eq!(eligible_group(9), Some(Group::A));
        assert_eq!(eligible_group(10), None);
        assert_eq!(eligible_group(11), None);
        assert_eq!(eligible_group(12), Some(Group::B));
        assert_eq!(eligible_group(13), None);
        assert_eq!(eligible_group(14), None);
        assert_eq!(eligible_group(15), Some(Group::A));
        assert_eq!(eligible_group(16), None);
        assert_eq!(eligible_group(17), None);
        assert_eq!(eligible_group(18), Some(Group::B));
        assert_eq!(eligible_group(19), None);
        assert_eq!(eligible_group(20), None);
        assert_eq!(eligible_group(21), Some(Group::A));
        assert_eq!(eligible_group(22), None);
        assert_eq!(eligible_group(23), None);
        assert_eq!(eligible_group(24), Some(Group::B));
        assert_eq!(eligible_group(25), None);
        assert_eq!(eligible_group(26), None);
        assert_eq!(eligible_group(27), Some(Group::A));
        assert_eq!(eligible_group(28), None);
        assert_eq!(eligible_group(29), None);
        assert_eq!(eligible_group(30), Some(Group::B));
        assert_eq!(eligible_group(31), None);
        assert_eq!(eligible_group(32), None);
        assert_eq!(eligible_group(33), Some(Group::A));
        assert_eq!(eligible_group(34), None);
        assert_eq!(eligible_group(35), None);
        assert_eq!(eligible_group(36), Some(Group::B));
        assert_eq!(eligible_group(37), None);
        assert_eq!(eligible_group(38), None);
        assert_eq!(eligible_group(39), Some(Group::A));
        assert_eq!(eligible_group(40), None);
    }
}
