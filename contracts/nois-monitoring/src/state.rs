use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Timestamp};
use cw_storage_plus::{Item, Map};

pub const NOIS_PROXY: Item<Addr> = Item::new("nois_proxy");
pub const JOB_OUTCOMES: Map<&str, u8> = Map::new("dice_outcomes");

pub const JOB_REQUESTS: Map<&str, JobLifecycleRequest> = Map::new("job_requests");
pub const JOB_DELIVERIES: Map<&str, JobLifecycleDelivery> = Map::new("job_deliveries");

#[cw_serde]
pub struct JobLifecycleRequest {
    pub height: u64,
    pub tx_index: Option<u32>,
    /// Safety margin in nanoseconds
    pub safety_margin: u64,
    // Requested beacon with publish time > after
    pub after: Timestamp,
}

#[cw_serde]
pub struct JobLifecycleDelivery {
    // Publishing time of the delivered beacon. Must be > `after`.
    pub published: Timestamp,
    pub height: u64,
    pub tx_index: Option<u32>,
}
