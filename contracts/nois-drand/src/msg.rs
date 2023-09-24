use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, HexBinary, Timestamp, Uint128};

use crate::state::{Config, QueriedBeacon, QueriedBot, StoredSubmission};

#[cw_serde]
pub struct InstantiateMsg {
    pub manager: String,
    /// The lowest drand round this contracts accepts for verification and storage.
    pub min_round: u64,
    /// Bot incentive amount.
    ///
    /// A round submission currently consumes 620k. Using a gas limit of
    /// 700k and a gas cost of 0.05unois/gas this is a fee of 35000unois.
    /// Anything above that is a useful incentive.
    pub incentive_point_price: Uint128,
    /// Bot incentive denom
    pub incentive_denom: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Add drand beacon
    AddRound { round: u64, signature: HexBinary },
    /// Registers a bot using on the sender address of the message.
    /// A re-registation updates the information of the bot.
    RegisterBot { moniker: String },
    /// Sets the round to be an incentivized one.
    /// The round number must be >= min_round fon the config. Otherwise a round would be
    /// incentivized that cannot be submitted.
    /// Only privilaged addresses can do this, which is typically just the gateway.
    SetIncentivized { round: u64 },
    UpdateAllowlistBots {
        add: Vec<String>,
        remove: Vec<String>,
    },
    SetConfig {
        manager: Option<String>,
        gateway: Option<String>,
        min_round: Option<u64>,
        incentive_point_price: Option<Uint128>,
        incentive_denom: Option<String>,
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
    #[returns(IsIncentivizedResponse)]
    IsIncentivized {
        /// The address of the sender (bot) for which the requested rounds are incentivized or not.
        sender: String,
        rounds: Vec<u64>,
    },
    #[returns(SubmissionsResponse)]
    Submissions { round: u64 },
    /// Get a specific bot by address
    #[returns(BotResponse)]
    Bot { address: String },
    /// Gets registered bots
    #[returns(BotsResponse)]
    Bots {},
    /// Returns the list of allowed bot addresses
    #[returns(AllowlistResponse)]
    Allowlist {},
    /// Returns if the given bot addresses is in the allowlist.
    #[returns(IsAllowlistedResponse)]
    IsAllowlisted {
        /// The address of the bot
        bot: String,
    },
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

#[cw_serde]
pub struct AllowlistResponse {
    /// List of bot addresses
    pub allowed: Vec<String>,
}

#[cw_serde]
pub struct IsAllowlistedResponse {
    pub listed: bool,
}

/// Like StoredSubmission but plus bot address
#[cw_serde]
pub struct QueriedSubmission {
    /// Address of the bot
    pub bot: Addr,
    /// Submission time (block time)
    pub time: Timestamp,
    /// Submission block height
    pub height: u64,
    /// Submission tx index
    pub tx_index: Option<u32>,
}

impl QueriedSubmission {
    pub fn make(stored: StoredSubmission, bot_address: Addr) -> Self {
        let StoredSubmission {
            pos: _, // not needed since we sort in `query_submissions`
            time,
            height,
            tx_index,
        } = stored;
        Self {
            bot: bot_address,
            time,
            height,
            tx_index,
        }
    }
}

#[cw_serde]
pub struct IsIncentivizedResponse {
    /// A list of results, one element pre requested round
    pub incentivized: Vec<bool>,
}

#[cw_serde]
pub struct SubmissionsResponse {
    pub round: u64,
    pub submissions: Vec<QueriedSubmission>,
}

#[cw_serde]
pub enum NoisGatewayExecuteMsg {
    /// Add drand beacon
    AddVerifiedRound {
        round: u64,
        randomness: HexBinary,
        is_verifying_tx: bool,
    },
}
