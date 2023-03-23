use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, Uint128};

use crate::state::Config;

#[cw_serde]
pub struct InstantiateMsg {
    /// The multisig wallet that controls the incentive and delegation.
    /// The manager is not be able to withdraw funds from the contract
    /// because no such functionality exists.
    pub manager: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    Delegate {
        /// The validator address
        addr: String,
        /// The amount in unois
        amount: Uint128,
    },
    Undelegate {
        /// The validator address
        addr: String,
        /// The amount in unois
        amount: Uint128,
    },
    Redelegate {
        /// The validator address to redelegate from
        src_addr: String,
        /// The validator address to redelegate to
        dest_addr: String,
        /// The amount in unois
        amount: Uint128,
    },
    ClaimRewards {
        /// The validator address
        addr: String,
    },
    SetDrandAddr {
        /// New address of the nois-drand contract
        addr: String,
    },
    SendFundsToDrand {
        funds: Coin,
    },
    SetManagerAddr {
        manager: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get the config state
    #[returns(ConfigResponse)]
    Config {},
}

// We define a custom struct for each query response
pub type ConfigResponse = Config;
