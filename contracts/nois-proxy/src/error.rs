use thiserror::Error;

use cosmwasm_std::StdError;

use nois_protocol::ChannelError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Job ID exceeds length limit.")]
    JobIdTooLong,

    #[error("Insufficient payment.")]
    InsufficientPayment,

    //
    // IBC
    //
    #[error("The nois-proxy contract must be on chain A of the connection. Try swapping A and B in the channel creation.")]
    MustBeChainA,

    #[error("Unsupported packet type.")]
    UnsupportedPacketType,

    #[error("Channel is not stored. Channel not yet established or closed.")]
    UnsetChannel,

    #[error("Channel was already created. Channel is immutable and cannot be overriden")]
    ChannelAlreadySet,

    #[error("Channel must not be closed.")]
    ChannelMustNotBeClosed,

    #[error("{0}")]
    ChannelError(#[from] ChannelError),
}
