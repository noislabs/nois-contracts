use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::Item;

#[cw_serde]
pub struct Config {
    /// The multisig wallet that controls the incentive and delegation.
    /// The manager is not be able to withdraw funds from the contract
    /// because no such functionality exists.
    pub manager: Addr,
    /// Address of the nois-drand contract. This is unset during instatiation
    /// but once set becomes immutable.
    pub drand: Option<Addr>,
}

pub const CONFIG: Item<Config> = Item::new("config");
