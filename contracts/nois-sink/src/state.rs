use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Timestamp};
use cw_storage_plus::{Item, Map};

// A history of all burns
//pub const ABSHES: (Addr;Uint128) = Map::new("beacons");
pub const ASHES: Map<u32, Ash> = Map::new("ashes");
pub const ASHES_COUNT: Item<u32> = Item::new("ashes_count");

#[cw_serde]
pub struct Ash {
    pub burner: Addr,
    pub amount: Coin,
    /// Point in time (block time) when the Ash was created
    pub burnt: Timestamp,
}
