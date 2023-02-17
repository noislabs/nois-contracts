use thiserror::Error;

use cosmwasm_std::StdError;

use nois_protocol::ChannelError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized. Contract is already set")]
    ContractAlreadySet,

    // Jobs
    #[error("Origin data exceeds length limit.")]
    OriginTooLong,

    #[error("Sender is unauthorized to add verified round.")]
    UnauthorizedAddVerifiedRound,

    // IBC
    #[error("{0}")]
    ChannelError(#[from] ChannelError),

    #[error("Cannot register over an existing channel")]
    ChannelAlreadyRegistered,
}
