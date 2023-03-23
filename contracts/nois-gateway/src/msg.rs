use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, HexBinary};

use crate::state::Config;

#[cw_serde]
pub struct InstantiateMsg {
    /// The prices of a randomness. List is to be interpreted as oneof,
    /// i.e. payment must be paid in one of those denominations.
    pub prices: Vec<Coin>,
    pub manager: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Add drand beacon
    AddVerifiedRound {
        round: u64,
        randomness: HexBinary,
        is_verifying_tx: bool,
    },
    /// Set the config
    SetConfig {
        manager: Option<String>,
        prices: Option<Vec<Coin>>,
        drand_addr: Option<String>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get the config state
    #[returns(ConfigResponse)]
    Config {},
    /// Gets basic statistics about jobs in this drand round.
    #[returns(DrandJobStatsResponse)]
    DrandJobStats { round: u64 },
}

// We define a custom struct for each query response
pub type ConfigResponse = Config;

#[cw_serde]
pub struct DrandJobStatsResponse {
    pub round: u64,
    /// Number of unprocessed jobs
    pub unprocessed: u32,
    /// Number of processed jobs
    pub processed: u32,
}
