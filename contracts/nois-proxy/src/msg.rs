use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, Timestamp, Uint128};
use nois::ProxyExecuteMsg;

use crate::state::{Config, OperationalMode};

#[cw_serde]
pub struct InstantiateMsg {
    /// The prices of a randomness. List is to be interpreted as oneof,
    /// i.e. payment must be paid in one of those denominations.
    /// If this list is empty, the user cannot pay. This can be used to put the
    /// contract out of service.
    pub prices: Vec<Coin>,
    pub manager: Option<String>,
    /// In test mode the min publish time calculation is detached from the clock.
    pub test_mode: bool,
    /// The amount of gas that the callback to the dapp can consume
    pub callback_gas_limit: u64,
    pub mode: OperationalMode,
}

#[cw_serde]
pub enum ExecuteMsg {
    // KEEP IN SYNC WITH ProxyExecuteMsg::GetNextRandomness
    GetNextRandomness {
        job_id: String,
    },
    // KEEP IN SYNC WITH ProxyExecuteMsg::GetRandomnessAfter
    GetRandomnessAfter {
        after: Timestamp,
        job_id: String,
    },
    /// Set the config
    SetConfig {
        manager: Option<String>,
        /// The prices of a randomness. List is to be interpreted as oneof,
        /// i.e. payment must be paid in one of those denominations.
        /// If this list is empty, the user cannot pay. This can be used to put the
        /// contract out of service.
        prices: Option<Vec<Coin>>,
        /// Address of the payment contract (on the other chain)
        payment: Option<String>,
        /// The amount of tokens the proxy sends for each randomness request to the Nois chain
        nois_beacon_price: Option<Uint128>,
        mode: Option<OperationalMode>,
    },
    // Withdraw the given amount to the withdrawal address
    Withdaw {
        denom: String,
        /// The amount of tokens to withdraw.
        /// If None, withdraw all available balance of the given denom.
        amount: Option<Uint128>,
        address: String,
    },
}

// Unused from implementation to bring up compiler errors when the
// types get out of sync.
impl From<ProxyExecuteMsg> for ExecuteMsg {
    fn from(source: ProxyExecuteMsg) -> ExecuteMsg {
        match source {
            ProxyExecuteMsg::GetNextRandomness { job_id } => {
                ExecuteMsg::GetNextRandomness { job_id }
            }
            ProxyExecuteMsg::GetRandomnessAfter { after, job_id } => {
                ExecuteMsg::GetRandomnessAfter { after, job_id }
            }
        }
    }
}

#[cw_serde]
pub enum SudoMsg {
    /// Withdraw the given amount to the withdrawal address
    #[cfg(feature = "governance_owned")]
    Withdaw {
        denom: String,
        /// The amount of tokens to withdraw.
        /// If None, withdraw all available balance of the given denom.
        amount: Option<Uint128>,
        address: String,
    },
    /// Withdraw the given amount to the community pool
    #[cfg(feature = "governance_owned")]
    WithdrawToCommunityPool {
        denom: String,
        /// The amount of tokens to withdraw.
        /// If None, withdraw all available balance of the given denom.
        amount: Option<Uint128>,
    },
    /// Set the config
    #[cfg(feature = "governance_owned")]
    SetConfig {
        manager: Option<String>,
        /// The prices of a randomness. List is to be interpreted as oneof,
        /// i.e. payment must be paid in one of those denominations.
        /// If this list is empty, the user cannot pay. This can be used to put the
        /// contract out of service.
        prices: Option<Vec<Coin>>,
        /// Address of the payment contract (on the other chain)
        payment: Option<String>,
        /// The amount of tokens the proxy sends for each randomness request to the Nois chain
        nois_beacon_price: Option<Uint128>,
        mode: Option<OperationalMode>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get the config state
    #[returns(ConfigResponse)]
    Config {},
    /// Get the prices.
    #[returns(PricesResponse)]
    Prices {},
    /// Get the price in one particular denom. Returns None when denom is not accepted.
    #[returns(PriceResponse)]
    Price { denom: String },
    /// Queries the local channel ID that is used to request beacons.
    /// This channel is unset when the proxy is instantiated and will be set when
    /// the channel is created. Once created, the value does not change anymore.
    #[returns(GatewayChannelResponse)]
    GatewayChannel {},
}

#[cw_serde]
pub struct ConfigResponse {
    pub config: Config,
}

#[cw_serde]
pub struct PricesResponse {
    /// Prices are encoded in a one-of list.
    pub prices: Vec<Coin>,
}

#[cw_serde]
pub struct PriceResponse {
    /// Price for the requested denom
    pub price: Option<Uint128>,
}

#[cw_serde]
pub struct GatewayChannelResponse {
    pub channel: Option<String>,
}

/// This struct contains information about the origin of the beacon request. It helps the
/// proxy to route the beacon response to the final destination.
/// The IBC communication between proxy and gateway does not need this information. It is
/// just passed along.
#[cw_serde]
pub struct RequestBeaconOrigin {
    /// The address of the dapp that requested the beacon. This is used by the proxy
    /// to send the callback.
    pub sender: String,
    pub job_id: String,
}
