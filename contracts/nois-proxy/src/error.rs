use thiserror::Error;

use cosmwasm_std::StdError;

use nois_ibc_protocol::ChannelError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Channel is not stored. Channel not yet established or closed.")]
    UnsetChannel,

    #[error("{0}")]
    ChannelError(#[from] ChannelError),

    #[error("No account for channel {0}")]
    UnregisteredChannel(String),

    #[error("remote account changed from {old} to {addr}")]
    RemoteAccountChanged { addr: String, old: String },

    #[error("Foreign error: {err}")]
    ForeignError { err: String },
}
