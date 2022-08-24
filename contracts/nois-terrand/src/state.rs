use cosmwasm_std::Binary;
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use nois_ibc_protocol::Beacon;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Config {
    pub drand_pubkey: Binary,
}

pub const CONFIG: Item<Config> = Item::new("config");

// A map from round number to drand beacon
pub const BEACONS: Map<u64, Beacon> = Map::new("beacons");
