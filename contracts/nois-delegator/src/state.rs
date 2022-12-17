use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::Item;

#[cw_serde]
pub struct Config {
    /// The multisig admin wallet that controls the incentive and delegation.
    /// The admin should not be able to withdraw funds from the contract
    pub admin_addr: Addr,
    // Nois Oracle contract. Not set during instatiation but once set should become immutable
    pub nois_oracle_contract_addr: Option<Addr>,
}

pub const CONFIG: Item<Config> = Item::new("config");
