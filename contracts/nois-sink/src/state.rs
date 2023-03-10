use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Timestamp};
use cw_storage_plus::{Item, Map};

/// A history of all burns.
/// This is a map from primary ID to the value.
pub const ASHES: Map<u32, Ash> = Map::new("ashes");
/// The last used primary ID
pub const ASHES_LAST_ID: Item<u32> = Item::new("ashes_id");

#[cw_serde]
pub struct Ash {
    pub burner: Option<Addr>,
    pub amount: Coin,
    /// Point in time (block time) when the Ash was created
    pub time: Timestamp,
}
