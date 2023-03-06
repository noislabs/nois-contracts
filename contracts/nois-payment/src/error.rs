use thiserror::Error;

use cosmwasm_std::StdError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Sending funds via wasm module is not allowed")]
    DontSendFunds,

    #[error("Invalid Address")]
    InvalidAddress,
}
