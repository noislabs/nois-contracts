use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::HexBinary;

use crate::state::Config;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    /// Add drand beacon
    AddVerifiedRound { round: u64, randomness: HexBinary },
    /// Set the address of the drand beackend contract
    SetDrandAddr { addr: String },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get the config state
    #[returns(ConfigResponse)]
    Config {},
    /// Gets basic statistics about jobs in this round.
    #[returns(JobStatsResponse)]
    JobStats { round: u64 },
}

// We define a custom struct for each query response
pub type ConfigResponse = Config;

#[cw_serde]
pub struct JobStatsResponse {
    pub round: u64,
    /// Number of unprocessed jobs
    pub unprocessed: u32,
    /// Number of processed jobs
    pub processed: u32,
}
