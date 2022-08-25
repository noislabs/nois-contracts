use cosmwasm_std::Decimal;
use cw_storage_plus::{Item, Map};

pub const NOIS_PROXY: Item<String> = Item::new("nois_proxy");

/// Map from callback ID to result
pub const RESULTS: Map<&str, Decimal> = Map::new("results");
