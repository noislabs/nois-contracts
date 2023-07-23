use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin};
use cw_storage_plus::Item;

#[cw_serde]
pub struct Config {
    /// The address of the drand contract.
    pub drand: Option<Addr>,
    /// Addresses which are trusted for providing randomness to the gateway.
    /// None and an empty vector means the same. This is just an Option for compatibility
    /// with previous versions of the contract.
    pub trusted_sources: Option<Vec<Addr>>,
    /// Manager to set the price and drand address
    pub manager: Addr,
    /// The price to pay in order to register the randomness job
    pub price: Coin,
    /// The code ID of the payment contract to be instantatiated
    pub payment_code_id: u64,
    /// An amount the gateway sends to the payment contract during instantiation.
    /// Used for testing only to avoid draining the gateway's balance by opening channels.
    /// Use None or 0unois to disable.
    pub payment_initial_funds: Option<Coin>,
    /// Address of the Nois sink
    pub sink: Addr,
}

pub const CONFIG: Item<Config> = Item::new("config");
