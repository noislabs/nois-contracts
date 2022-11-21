use thiserror::Error;

use cosmwasm_std::StdError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized.")]
    Unauthorized,

    #[error("Unauthorized. Contract is already set")]
    ContractAlreadySet,

    #[error("Invalid address")]
    InvalidAddress,

    #[error("The nois oracle contract address is still unset. Consider setting that first")]
    NoisOracleContractAddressUnset,
}
