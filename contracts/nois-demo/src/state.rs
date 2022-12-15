use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal};
use cw_storage_plus::{Item, Map};

pub const NOIS_PROXY: Item<Addr> = Item::new("nois_proxy");

/// Map from callback ID to result
pub const RESULTS: Map<&str, Decimal> = Map::new("results");
