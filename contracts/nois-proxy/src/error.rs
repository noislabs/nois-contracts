use thiserror::Error;

use cosmwasm_std::StdError;

use nois_protocol::ChannelError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Job ID exceeds length limit.")]
    JobIdTooLong,

    #[error("Channel is not stored. Channel not yet established or closed.")]
    UnsetChannel,

    #[error("Channel was already created. Channel is immutable and cannot be overriden")]
    ChannelAlreadySet,

    #[error("Channel must not be closed.")]
    ChannelMustNotBeClosed,

    #[error("{0}")]
    ChannelError(#[from] ChannelError),

    #[error("No account for channel {0}")]
    UnregisteredChannel(String),

    #[error("remote account changed from {old} to {addr}")]
    RemoteAccountChanged { addr: String, old: String },

    #[error("Foreign error: {err}")]
    ForeignError { err: String },
}
