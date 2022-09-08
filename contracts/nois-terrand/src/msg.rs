use cosmwasm_schema::{cw_serde, QueryResponses};
use nois_protocol::Data;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::{Config, VerifiedBeacon};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {
    /// In test mode the next round calculation is detached from the clock.
    pub test_mode: bool,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Add drand beacon
    AddRound {
        round: u64,
        previous_signature: Data,
        signature: Data,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get the config state
    #[returns(ConfigResponse)]
    Config {},
    /// Get a specific drand round
    #[returns(BeaconReponse)]
    Beacon { round: u64 },
    /// Get the last randomness
    #[returns(LatestRandomResponse)]
    LatestDrand {},
}

// We define a custom struct for each query response
pub type ConfigResponse = Config;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema, Default)]
pub struct BeaconReponse {
    pub beacon: Option<VerifiedBeacon>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct LatestRandomResponse {
    pub round: u64,
    pub beacon: VerifiedBeacon,
}
