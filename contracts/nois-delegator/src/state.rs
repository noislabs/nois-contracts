use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::Item;

#[cw_serde]
pub struct Config {
    /// The multisig admin wallet that controls the incentive and delegation.
    /// The admin should not be able to withdraw funds from the contract
    pub admin_addr: String,
    /// Bot incentive amount.
    ///
    /// A round submission currently consumes 620k. Using a gas limit of
    /// 700k and a gas cost of 0.05unois/gas this is a fee of 35000unois.
    /// Anything above that is a useful incentive.
    pub incentive_amount: Uint128,
    /// Bot incentive denom
    pub incentive_denom: String,
    // The staking, unbonding, redelegating, claim denom. It can be the same as the incentive denom
    pub staking_denom: String,
    // Nois Oracle contract. Not set during instatiation but once set should become immutable
    pub nois_oracle_contract_addr: Option<Addr>,
}

pub const CONFIG: Item<Config> = Item::new("config");
