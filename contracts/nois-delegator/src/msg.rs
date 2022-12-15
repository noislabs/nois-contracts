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
    SendFundsToOracle {
        amount: Uint128,
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
