use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const SINK: Item<Addr> = Item::new("nois_sink");
pub const COMMUNITY_POOL: Item<Addr> = Item::new("nois_community_pool");
