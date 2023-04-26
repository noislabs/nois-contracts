use std::collections::BTreeMap;

use crate::error::ContractError;
use cosmwasm_std::Coin;
use nois::MAX_JOB_ID_LEN;

pub fn validate_job_id(job_id: &str) -> Result<(), ContractError> {
    if job_id.len() > MAX_JOB_ID_LEN {
        Err(ContractError::JobIdTooLong)
    } else {
        Ok(())
    }
}

/// Checks if provided funds are sufficient to pay the price in one of the
/// supported denoms. Payment cannot be split across multiple denoms. Extra funds
/// are ignored.
///
/// When `prices` is an empty list the user cannot pay because there is no possible
/// denomination in which they could do that. This can be desired in case the cantract
/// does not want to accapt any payment (i.e. is closed).
pub fn validate_payment(prices: &[Coin], funds: &[Coin]) -> Result<(), ContractError> {
    if prices.is_empty() {
        return Err(ContractError::NoPaymentOption);
    }

    let prices = BTreeMap::from_iter(prices.iter().map(|c| (c.denom.clone(), c.amount)));
    for fund in funds {
        if let Some(price) = prices.get(&fund.denom) {
            // user can pay in this provided denom
            if fund.amount >= *price {
                return Ok(());
            }
        }
    }
    Err(ContractError::InsufficientPayment)
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{coin, coins};

    use super::*;

    #[test]
    fn validate_job_id_works() {
        // We have no good reason to disallow empty
        validate_job_id("").unwrap();
        validate_job_id("abc").unwrap();
        validate_job_id("abcabcabcabcabcaabc2bcabcabcabcabcabcabc").unwrap();
        // a sha256 hash
        validate_job_id("cb480eb3697f39db828d9efa021abe681bfcd72e23894019b8ddb1ab94039081")
            .unwrap();

        let err =
            validate_job_id("abcabcabcabcabca_asfsdfsdgsdgbcbcabcabcabc234t34t3t34gabcabcabc41")
                .unwrap_err();
        assert!(matches!(err, ContractError::JobIdTooLong));
    }

    #[test]
    fn validate_payment_works() {
        // No payment option
        let prices = Vec::<Coin>::new();
        let err = validate_payment(&prices, &[coin(100, "bucks")]).unwrap_err();
        assert!(matches!(err, ContractError::NoPaymentOption));
        let err = validate_payment(&prices, &[]).unwrap_err();
        assert!(matches!(err, ContractError::NoPaymentOption));

        // Single payment option
        let prices = coins(100, "bucks");
        validate_payment(&prices, &[coin(100, "bucks")]).unwrap();
        validate_payment(&prices, &[coin(150, "bucks")]).unwrap();
        validate_payment(&prices, &[coin(100, "bucks"), coin(20, "ujunox")]).unwrap();
        validate_payment(&prices, &[coin(20, "ujunox"), coin(100, "bucks")]).unwrap();
        validate_payment(
            &prices,
            &[coin(20, "ujunox"), coin(100, "bucks"), coin(55, "uluna")],
        )
        .unwrap();

        let err = validate_payment(&prices, &[coin(99, "bucks")]).unwrap_err();
        assert!(matches!(err, ContractError::InsufficientPayment));
        let err = validate_payment(&prices, &[coin(99, "bucks"), coin(17, "bucks")]).unwrap_err();
        assert!(matches!(err, ContractError::InsufficientPayment));
        let err = validate_payment(&prices, &[]).unwrap_err();
        assert!(matches!(err, ContractError::InsufficientPayment));
        let err = validate_payment(&prices, &[coin(99, "bucks"), coin(200, "gold")]).unwrap_err();
        assert!(matches!(err, ContractError::InsufficientPayment));

        // Multi payment option
        let prices = vec![coin(100, "bucks"), coin(20, "sand")];
        validate_payment(&prices, &[coin(100, "bucks")]).unwrap();
        validate_payment(&prices, &[coin(20, "sand")]).unwrap();
        validate_payment(&prices, &[coin(150, "bucks")]).unwrap();
        validate_payment(&prices, &[coin(50, "sand")]).unwrap();
        validate_payment(&prices, &[coin(100, "bucks"), coin(20, "ujunox")]).unwrap();
        validate_payment(&prices, &[coin(20, "ujunox"), coin(100, "bucks")]).unwrap();
        validate_payment(
            &prices,
            &[coin(20, "ujunox"), coin(100, "bucks"), coin(55, "uluna")],
        )
        .unwrap();

        let err = validate_payment(&prices, &[coin(99, "bucks")]).unwrap_err();
        assert!(matches!(err, ContractError::InsufficientPayment));
        let err = validate_payment(&prices, &[coin(99, "bucks"), coin(17, "bucks")]).unwrap_err();
        assert!(matches!(err, ContractError::InsufficientPayment));
        let err = validate_payment(&prices, &[coin(99, "bucks"), coin(17, "sand")]).unwrap_err();
        assert!(matches!(err, ContractError::InsufficientPayment));
        let err = validate_payment(&prices, &[]).unwrap_err();
        assert!(matches!(err, ContractError::InsufficientPayment));
        let err = validate_payment(&prices, &[coin(99, "bucks"), coin(200, "gold")]).unwrap_err();
        assert!(matches!(err, ContractError::InsufficientPayment));
    }
}
