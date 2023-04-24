use thiserror::Error;

use cosmwasm_std::StdError;

#[derive(Error, Debug, PartialEq)]
#[non_exhaustive]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Wrong coin denom")]
    WrongDenom,

    #[error("Too many coins sent")]
    TooManyCoins,

    #[error("No coins sent")]
    NoCoins,

    #[error("This message doesn't accept funds. You might want to burn using the burn message instead of burn_native")]
    NonPayableMessage,

    #[error("There are no funds in the sink contract to burn")]
    NoFundsToBurn,
}
