use cosmwasm_std::Decimal;
use cw_storage_plus::{Item, Map};

pub const NOIS_PROXY: Item<String> = Item::new("nois_proxy");

/// Map from ID to result
pub const RESULTS: Map<&str, String> = Map::new("results");
pub const LATEST_RESULT: Item<Decimal> = Item::new("latest_result");
