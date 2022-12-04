use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;

use crate::state::Config;

#[cw_serde]
pub struct InstantiateMsg {
    /// The multisig admin wallet that controls the incentive and delegation.
    /// The admin should not be able to withdraw funds from the contract
    pub admin_addr: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Add drand beacon
    IncentiviseBot {
        addr: String,
        /// Bot incentive amount.
        ///
        /// A round submission currently consumes 620k. Using a gas limit of
        /// 700k and a gas cost of 0.05unois/gas this is a fee of 35000unois.
        /// Anything above that is a useful incentive.
        incentive_amount: Uint128,
        /// Bot incentive denom
        incentive_denom: String,
    },
    /// Registers a bot using on the sender address of the message.
    /// A re-registation updates the information of the bot.
    Delegate {
        addr: String,
        amount: Uint128,
    },
    Undelegate {
        addr: String,
        amount: Uint128,
    },
    Redelegate {
        src_addr: String,
        dest_addr: String,
        amount: Uint128,
    },
    ClaimRewards {
        addr: String,
    },
    SetNoisOracleContractAddr {
        addr: String,
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