use cosmwasm_std::Binary;
use nois_ibc_protocol::{Beacon, Data};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::Config;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {
    /// The public key of the drand network
    pub pubkey: Binary,
    /// In test mode the next round calculation is detached from the clock.
    pub test_mode: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Add drand beacon
    AddRound {
        round: u64,
        previous_signature: Data,
        signature: Data,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Get the config state
    Config {},
    /// Get the last randomness
    LatestDrand {},
    /// Get a specific drand round
    Beacon { round: u64 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema, Default)]
pub struct BeaconReponse {
    pub beacon: Option<Beacon>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct LatestRandomResponse {
    pub round: u64,
    pub randomness: Data,
}

// We define a custom struct for each query response
pub type ConfigResponse = Config;
