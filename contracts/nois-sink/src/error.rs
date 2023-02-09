use thiserror::Error;

use cosmwasm_std::StdError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Wrong coin denom")]
    WrongDenom,

    #[error("Too many coins sent")]
    TooManyCoins,
}
