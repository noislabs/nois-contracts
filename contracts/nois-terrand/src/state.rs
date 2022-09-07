use cosmwasm_std::Timestamp;
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use nois_protocol::Data;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Config {
    /// In test mode the next round calculation is detached from the clock.
    pub test_mode: bool,
}

pub const CONFIG: Item<Config> = Item::new("config");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct VerifiedBeacon {
    pub published: Timestamp,
    pub verified: Timestamp,
    /// The sha256(signature) in lower case hex
    pub randomness: Data,
}

// A map from round number to drand beacon
pub const BEACONS: Map<u64, VerifiedBeacon> = Map::new("beacons");

pub const TEST_MODE_NEXT_ROUND: Item<u64> = Item::new("test_mode_next_round");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Job {
    /// A RNG specific randomness source identifier, e.g. `drand:<network id>:<round>`
    pub source_id: String,
    // The channel the job came from and we have to send the response to
    pub channel: String,
    // contract address on the app chain
    pub sender: String,
    // contract address on the app chain
    pub callback_id: Option<String>,
}

// Unprocessed drand jobs that are waiting for the correct round to come in
pub const DRAND_JOBS: Map<u64, Vec<Job>> = Map::new("drand_jobs");
