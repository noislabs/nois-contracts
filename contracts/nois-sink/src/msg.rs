use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin, Timestamp};

use crate::state::Ash;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    /// Burns the tokens that are sent as `funds` with this message
    Burn {},
    /// Burns all unois owned by the contract. This can be used
    /// if tokens were sent here using bank sends.
    BurnBalance {},
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

/// Like Ash but plus id
#[cw_serde]
pub struct QueriedAsh {
    pub id: u32,
    pub burner: Option<Addr>,
    pub amount: Coin,
    /// Point in time (block time) when the Ash was created
    pub time: Timestamp,
}

impl QueriedAsh {
    pub fn make(ash: Ash, id: u32) -> Self {
        let Ash {
            burner,
            amount,
            time,
        } = ash;
        Self {
            id,
            burner,
            amount,
            time,
        }
    }
}

#[cw_serde]
pub struct AshesResponse {
    pub ashes: Vec<QueriedAsh>,
}
