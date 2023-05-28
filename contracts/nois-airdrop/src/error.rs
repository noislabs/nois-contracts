use hex::FromHexError;
use thiserror::Error;

use cosmwasm_std::StdError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Hex(#[from] FromHexError),

    #[error("Unauthorized.")]
    Unauthorized,

    #[error("Proxy address is not valid")]
    InvalidProxyAddress,

    #[error("Merkle is immutable.")]
    MerkleImmutable,

    #[error("Register the Merkle root before requesting randomness")]
    MerkleRootAbsent,

    #[error("Invalid input")]
    InvalidInput {},

    #[error("Already claimed")]
    Claimed {},

    #[error("Wrong length")]
    WrongLength {},

    #[error("Verification failed")]
    VerificationFailed {},

    #[error("The sender is not randomly eligible for the randdrop")]
    NotRandomlyEligible {},

    #[error("The claiming phase didnot start. The random beacon is yet to be fetched")]
    RandomnessUnavailable {},

    #[error("Cannot migrate from different contract type: {previous_contract}")]
    CannotMigrate { previous_contract: String },

    // callback should only be allowed to be called by the proxy contract
    // otherwise anyone can cut the randomness workflow and cheat the randomness
    #[error("Unauthorized Receive execution")]
    UnauthorizedReceive,

    #[error("Received invalid randomness")]
    InvalidRandomness,

    #[error("Immutable Randomness")]
    ImmutableRandomness,
}
