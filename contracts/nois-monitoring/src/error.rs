use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Proxy address is not valid")]
    InvalidProxyAddress,

    #[error("Job ID exceeds length limit.")]
    JobIdTooLong,

    #[error("Round already present")]
    JobIdAlreadyPresent,

    //callback should only be allowed to be called by the proxy contract
    //otherwise anyone can cut the randomness workflow and cheat the randomness
    #[error("Unauthorized Receive execution")]
    UnauthorizedReceive,

    #[error("Received invalid randomness")]
    InvalidRandomness,
}
