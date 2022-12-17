use thiserror::Error;

use cosmwasm_std::StdError;

use nois_protocol::ChannelError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    // Bots
    #[error("Moniker must not be empty.")]
    MonikerEmpty,

    #[error("Unauthorized.")]
    Unauthorized,

    #[error("Invalid Address.")]
    InvalidAddr,

    #[error("Moniker exceeds length limit.")]
    MonikerTooLong,

    // Jobs
    #[error("Job ID exceeds length limit.")]
    JobIdTooLong,

    // IBC
    #[error("{0}")]
    ChannelError(#[from] ChannelError),

    #[error("Cannot register over an existing channel")]
    ChannelAlreadyRegistered,

    // Other
    #[error("Invalid reply id")]
    InvalidReplyId,

    #[error("Invalid public key")]
    InvalidPubkey,

    #[error("Round {round} lower than min round {min_round}")]
    RoundTooLow { round: u64, min_round: u64 },

    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Invalid Delegator/Incentive Contract")]
    InvalidDelegatorAddress,

    #[error("Foreign error: {err}")]
    ForeignError { err: String },

    #[error("A submission for the same round from the same bot exists already")]
    SubmissionExists,
}
