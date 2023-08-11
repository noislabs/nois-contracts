use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::Map;

use super::TopKey;

#[cw_serde]
pub struct Customer {
    /// The payment contract address
    pub payment: Addr,
    /// Number of beacons requested in total
    pub requested_beacons: u64,
}

/// A map from channel ID to customer information
pub const CUSTOMERS: Map<&str, Customer> = Map::new(TopKey::Customers.as_str());
