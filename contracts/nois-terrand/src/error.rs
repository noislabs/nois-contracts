use thiserror::Error;

use cosmwasm_std::StdError;

use nois_ibc_protocol::ChannelError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    ChannelError(#[from] ChannelError),

    #[error("Cannot register over an existing channel")]
    ChannelAlreadyRegistered,

    #[error("Invalid reply id")]
    InvalidReplyId,

    #[error("Invalid public key")]
    InvalidPubkey,

    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Foreign error: {err}")]
    ForeignError { err: String },
}
