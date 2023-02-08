use cosmwasm_schema::{cw_serde, QueryResponses};

use crate::state::Ash;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    /// Add drand beacon
    Burn {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Gets ashes in ascending order (old to new)
    #[returns(AshesResponse)]
    AshesAsc {
        // If set only burn operation greater than this value are returned
        start_after: Option<u32>,
        /// The max number of entries returned. If you set this too high, your query runs out of gas.
        /// When unset, an implementation defined default will be used.
        limit: Option<u32>,
    },
    /// Gets ashes in descending order (new to old)
    #[returns(AshesResponse)]
    AshesDesc {
        // If set only burn operation lower than this value are returned
        start_after: Option<u32>,
        /// The max number of entries returned. If you set this too high, your query runs out of gas.
        /// When unset, an implementation defined default will be used.
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct AshesResponse {
    pub ashes: Vec<Ash>,
}
