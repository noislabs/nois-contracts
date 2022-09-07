use nois_protocol::Data;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::{Config, VerifiedBeacon};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {
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
    pub beacon: Option<VerifiedBeacon>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct LatestRandomResponse {
    pub round: u64,
    pub beacon: VerifiedBeacon,
}

// We define a custom struct for each query response
pub type ConfigResponse = Config;
