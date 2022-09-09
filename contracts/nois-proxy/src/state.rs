use cosmwasm_schema::cw_serde;
use cw_storage_plus::Item;

#[cw_serde]
pub struct Config {}

pub const CONFIG: Item<Config> = Item::new("config");

pub const ORACLE_CHANNEL: Item<String> = Item::new("oracle_channel");
