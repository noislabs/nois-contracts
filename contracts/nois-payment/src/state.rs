use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::Item;

#[cw_serde]
pub struct Config {
    /// The address of the sink contract to burn the used tokens.
    pub sink: Addr,
    /// The address of the community pool.
    pub community_pool: Addr,
    /// The address of nois-gateway
    pub gateway: Addr,
}

pub const CONFIG: Item<Config> = Item::new("config");
