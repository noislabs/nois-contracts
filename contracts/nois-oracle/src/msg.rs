use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{HexBinary, Uint128};

use crate::state::Config;

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
