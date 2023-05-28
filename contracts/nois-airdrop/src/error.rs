use hex::FromHexError;
use thiserror::Error;

use cosmwasm_std::{StdError, Timestamp};

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

    #[error("The claiming phase did not start. The random beacon is yet to be fetched")]
    RandomnessUnavailable {},

    #[error("Cannot migrate from different contract type: {previous_contract}")]
    CannotMigrate { previous_contract: String },

    // callback should only be allowed to be called by the proxy contract
    // otherwise anyone can cut the randomness workflow and cheat the randomness
    #[error("Unauthorized Receive execution")]
    UnauthorizedReceive,

    #[error("Requesting randomness {random_beacon_after} in the past compared to  {block_time}. This is not safe, make sure the timestamp is in the future and in nanoseconds")]
    RandomAfterIsInThePast {
        block_time: Timestamp,
        random_beacon_after: Timestamp,
    },

    #[error(
        "Requesting randomness is too much in the future, max allowed is {max_allowed_beacon_time}"
    )]
    RandomAfterIsTooMuchInTheFuture { max_allowed_beacon_time: Timestamp },

    #[error("Received invalid randomness")]
    InvalidRandomness,

    #[error("Immutable Randomness")]
    ImmutableRandomness,
}
