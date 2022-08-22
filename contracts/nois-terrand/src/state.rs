use cosmwasm_std::Binary;
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use nois_ibc_protocol::Beacon;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub drand_public_key: Binary,
}

pub const CONFIG: Item<Config> = Item::new("config");

// A map from round number to drand beacon
pub const ROUNDS: Map<u64, Beacon> = Map::new("rounds");
