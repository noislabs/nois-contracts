use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Config {
    /// manager If None set, contract is frozen.
    pub manager: Option<Addr>,
    // The address to which withdrawals will be made after the claim is finished
    pub withdrawal_address: Addr,
}

#[cw_serde]
pub struct RandomnessParams {
    // The address of the nois-proxy contract deployed onthe same chain as this contract
    pub nois_proxy: Addr,
    // The randomness beacon received from the proxy
    pub nois_randomness: Option<[u8; 32]>,
    // If the randomness has already been requested
    pub requested: bool,
}

pub const CONFIG_KEY: &str = "config";
pub const CONFIG: Item<Config> = Item::new(CONFIG_KEY);

pub const MERKLE_ROOT_PREFIX: &str = "merkle_root";
pub const MERKLE_ROOT: Item<String> = Item::new(MERKLE_ROOT_PREFIX);

pub const CLAIM_PREFIX: &str = "claim";
pub const CLAIM: Map<Addr, bool> = Map::new(CLAIM_PREFIX);

pub const NOIS_RANDOMNESS: Item<RandomnessParams> = Item::new("nois_randomness");
