use thiserror::Error;

use cosmwasm_std::StdError;

use nois_protocol::ChannelError;

#[derive(Error, Debug, PartialEq)]
#[non_exhaustive]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized,

    // Payment
    #[error("Code ID does not exist: {code_id}")]
    CodeIdDoesNotExist { code_id: u64 },

    // Jobs
    #[error("Origin data exceeds length limit.")]
    OriginTooLong,

    #[error("Sender is unauthorized to add verified round.")]
    UnauthorizedAddVerifiedRound,

    // IBC
    #[error("The nois-gateway contract must be on chain B of the connection. Try swapping A and B in the channel creation.")]
    MustBeChainB,

    #[error("Unsupported packet type.")]
    UnsupportedPacketType,

    #[error("{0}")]
    ChannelError(#[from] ChannelError),

    #[error("Cannot register over an existing channel")]
    ChannelAlreadyRegistered,
}
