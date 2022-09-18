use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Timestamp};
use nois::HexBinary;

use crate::state::{Config, QueriedBeacon};

#[cw_serde]
pub struct InstantiateMsg {
    /// In test mode the next round calculation is detached from the clock.
    pub test_mode: bool,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Add drand beacon
    AddRound {
        round: u64,
        previous_signature: HexBinary,
        signature: HexBinary,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get the config state
    #[returns(ConfigResponse)]
    Config {},
    /// Get a specific drand round
    #[returns(BeaconResponse)]
    Beacon { round: u64 },
    /// Gets beacons in ascending order (old to new)
    #[returns(BeaconsResponse)]
    BeaconsAsc {
        // If set only rounds greater than this value are returned
        start_after: Option<u64>,
        /// The max number of entries returned. If you set this too high, your query runs out of gas.
        /// When unset, an implementation defined default will be used.
        limit: Option<u32>,
    },
    /// Gets beacons in descending order (new to old)
    #[returns(BeaconsResponse)]
    BeaconsDesc {
        // If set only rounds smaller than this value are returned
        start_after: Option<u64>,
        /// The max number of entries returned. If you set this too high, your query runs out of gas.
        /// When unset, an implementation defined default will be used.
        limit: Option<u32>,
    },
    #[returns(SubmissionsResponse)]
    Submissions { round: u64 },
}

// We define a custom struct for each query response
pub type ConfigResponse = Config;

#[cw_serde]
pub struct BeaconResponse {
    pub beacon: Option<QueriedBeacon>,
}

#[cw_serde]
pub struct BeaconsResponse {
    pub beacons: Vec<QueriedBeacon>,
}

#[cw_serde]
pub struct Submission {
    /// Address of the bot
    pub bot: Addr,
    /// Submission time
    pub time: Timestamp,
}

#[cw_serde]
pub struct SubmissionsResponse {
    pub round: u64,
    pub submissions: Vec<Submission>,
}
