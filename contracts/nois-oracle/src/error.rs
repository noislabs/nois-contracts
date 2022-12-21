use thiserror::Error;

use cosmwasm_std::StdError;

use nois_protocol::ChannelError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized.")]
    Unauthorized,

    #[error("Unauthorized. Contract is already set")]
    ContractAlreadySet,

    // Jobs
    #[error("Job ID exceeds length limit.")]
    JobIdTooLong,

    #[error("Sender is unauthorized to add verified round.")]
    UnauthorizedAddVerifiedRound,

    // IBC
    #[error("{0}")]
    ChannelError(#[from] ChannelError),

    #[error("Cannot register over an existing channel")]
    ChannelAlreadyRegistered,

    #[error("Foreign error: {err}")]
    ForeignError { err: String },
}
