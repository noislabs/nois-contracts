use thiserror::Error;

use cosmwasm_std::StdError;

#[derive(Error, Debug, PartialEq)]
#[non_exhaustive]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    // Bots
    #[error("Moniker must not be empty.")]
    MonikerEmpty,

    #[error("Unauthorized.")]
    Unauthorized,

    #[error("Unauthorized. Contract is already set")]
    ContractAlreadySet,

    #[error("Moniker exceeds length limit.")]
    MonikerTooLong,

    // Other
    #[error("Invalid public key")]
    InvalidPubkey,

    #[error("Round {round} lower than min round {min_round}")]
    RoundTooLow { round: u64, min_round: u64 },

    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Signature does not match the contract's state")]
    SignatureDoesNotMatchState,

    #[error("A submission for the same round from the same bot exists already")]
    SubmissionExists,
}
