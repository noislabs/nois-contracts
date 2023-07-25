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
    /// This is for testing only. Setting the field to `None` equals `Some(false)`.
    /// Using an optional type allows omitting the field in JSON instantiation messages.
    pub test_mode: Option<bool>,
    /// The amount of gas that the callback to the dapp can consume
    pub callback_gas_limit: u64,
    pub mode: OperationalMode,
    /// Toggle caller allowlist allowed to get randomness.
    /// This is optional and can be omitted. Defaults to false.
    pub allowlist_enabled: Option<bool>,
    /// List of addresses allowed to get randomness.
    /// This is optional and can be omitted. Defaults to an empty list.
    pub allowlist: Option<Vec<String>>,
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
    // This will schedule a job to call the dapp contract at a specific date
    ScheduleJobAt {
        after: Timestamp,
        job_id: String,
        // if the callback_addr is None, default to info.sender
        callback_addr: Option<String>,
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
        /// The amount of gas that the callback to the dapp can consume
        callback_gas_limit: Option<u64>,
        mode: Option<OperationalMode>,
        /// Toggle address allowlist to get randomness. When enabled, the allowlist is checked.
        /// Otherwise the allowlist entries are ignored.
        allowlist_enabled: Option<bool>,
        /// The minimal value for `after` when requesting a beacon.
        /// This aims to counter accidental misusage. Not all values in the allowed range are reasonable.
        min_after: Option<Timestamp>,
        /// The maximum value for `after` when requesting a beacon.
        /// This aims to counter accidental misusage. Not all values in the allowed range are reasonable.
        max_after: Option<Timestamp>,
    },
    // Withdraw the given amount to the withdrawal address
    Withdraw {
        denom: String,
        /// The amount of tokens to withdraw.
        /// If None, withdraw all available balance of the given denom.
        amount: Option<Uint128>,
        address: String,
    },
    // Add or remove entries from the list of addresses allowed to get randomness.
    UpdateAllowlist {
        add: Vec<String>,
        remove: Vec<String>,
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
    Withdraw {
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
        /// The amount of gas that the callback to the dapp can consume
        callback_gas_limit: Option<u64>,
        mode: Option<OperationalMode>,
        /// Toggle address allowlist to get randomness. When enabled, the allowlist is checked.
        /// Otherwise the allowlist entries are ignored.
        allowlist_enabled: Option<bool>,
        /// The minimal value for `after` when requesting a beacon.
        /// This aims to counter accidental misusage. Not all values in the allowed range are reasonable.
        min_after: Option<Timestamp>,
        /// The maximum value for `after` when requesting a beacon.
        /// This aims to counter accidental misusage. Not all values in the allowed range are reasonable.
        max_after: Option<Timestamp>,
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
    /// Returns the list of allowed dapp addresses
    #[returns(AllowlistResponse)]
    Allowlist {},
    /// Queries whether the given address is part of the proxy's allowlist.
    /// Whether or not the allowlist is enabled must be queried via the config.
    #[returns(IsAllowlistedResponse)]
    IsAllowlisted { address: String },
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

#[cw_serde]
pub struct AllowlistResponse {
    /// List of addresses
    pub allowed: Vec<String>,
}

#[cw_serde]
pub struct IsAllowlistedResponse {
    /// Returns true if and only if the address is part of the proxy's allowlist.
    /// Whether or not the allowlist is enabled must be queried via the config.
    pub listed: bool,
}

/// This struct contains information about the origin of the beacon request. It helps the
/// proxy to route the beacon response to the final callback_contract.
/// The IBC communication between proxy and gateway does not need this information. It is
/// just passed along.
#[cw_serde]
pub struct RequestBeaconCallback {
    /// The address of the dapp that will receive the beacon. This is used by the proxy
    /// to send the callback.
    pub callback_contract: String,
    pub job_id: String,
}
