use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, HexBinary};
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Config {
    /// manager that can change the manager , register merkle or withdraw funds
    pub manager: Addr,
    pub denom: String,
    // The address of the nois-proxy contract deployed onthe same chain as this contract
    pub nois_proxy: Addr,
}

#[cw_serde]
pub struct RandomnessParams {
    // The randomness beacon received from the proxy
    pub nois_randomness: Option<[u8; 32]>,
    // If the randomness has already been requested
    pub requested: bool,
}

pub const CONFIG_KEY: &str = "config";
pub const CONFIG: Item<Config> = Item::new(CONFIG_KEY);

pub const MERKLE_ROOT_PREFIX: &str = "merkle_root";
pub const MERKLE_ROOT: Item<HexBinary> = Item::new(MERKLE_ROOT_PREFIX);

pub const CLAIMED_PREFIX: &str = "claimed";
pub const CLAIMED_VALUE: u8 = 1;
/// A map that stores claimed addresses. Think of this as a set. Any existing value means the value is cleamed.
pub const CLAIMED: Map<&Addr, u8> = Map::new(CLAIMED_PREFIX);

pub const NOIS_RANDOMNESS: Item<RandomnessParams> = Item::new("nois_randomness");
