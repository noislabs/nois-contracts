use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, HexBinary, Timestamp, Uint128};

use crate::state::{Config, QueriedBeacon, QueriedBot, StoredSubmission};

#[cw_serde]
pub struct InstantiateMsg {
    /// The lowest drand round this contracts accepts for verification and storage.
    pub min_round: u64,
    /// Bot incentive amount.
    ///
    /// A round submission currently consumes 620k. Using a gas limit of
    /// 700k and a gas cost of 0.05unois/gas this is a fee of 35000unois.
    /// Anything above that is a useful incentive.
    pub incentive_amount: Uint128,
    /// Bot incentive denom
    pub incentive_denom: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Add drand beacon
    AddRound {
        round: u64,
        previous_signature: HexBinary,
        signature: HexBinary,
    },
    /// Add drand beacon
    AddVerifiedRound { round: u64, randomness: HexBinary },
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
    /// Get a specific bot by address
    #[returns(BotResponse)]
    Bot { address: String },
    /// Gets registered bots
    #[returns(BotsResponse)]
    Bots {},
    #[returns(SubmissionsResponse)]
    Submissions { round: u64 },
    /// Gets basic statistics about jobs in this round.
    #[returns(JobStatsResponse)]
    JobStats { round: u64 },
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
pub struct BotResponse {
    pub bot: Option<QueriedBot>,
}

#[cw_serde]
pub struct BotsResponse {
    pub bots: Vec<QueriedBot>,
}

/// Like StoredSubmission but plus bot address
#[cw_serde]
pub struct QueriedSubmission {
    /// Address of the bot
    pub bot: Addr,
    /// Submission time
    pub time: Timestamp,
}

impl QueriedSubmission {
    pub fn make(stored: StoredSubmission, bot_address: Addr) -> Self {
        let StoredSubmission { time } = stored;
        Self {
            bot: bot_address,
            time,
        }
    }
}

#[cw_serde]
pub struct SubmissionsResponse {
    pub round: u64,
    pub submissions: Vec<QueriedSubmission>,
}

#[cw_serde]
pub struct JobStatsResponse {
    pub round: u64,
    /// Number of unprocessed jobs
    pub unprocessed: u32,
    /// Number of processed jobs
    pub processed: u32,
}
