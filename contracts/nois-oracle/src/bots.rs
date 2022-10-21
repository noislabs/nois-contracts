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
}
