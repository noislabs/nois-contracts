use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, Timestamp};
use nois::ProxyExecuteMsg;

use crate::state::Config;

#[cw_serde]
pub struct InstantiateMsg {
    pub withdrawal_address: String,
    /// In test mode the min publish time calculation is detached from the clock.
    pub test_mode: bool,
}

#[cw_serde]
pub enum ExecuteMsg {
    // KEEP IN SYNC WITH ProxyExecuteMsg::GetNextRandomness
    GetNextRandomness { job_id: String },
    // KEEP IN SYNC WITH ProxyExecuteMsg::GetRandomnessAfter
    GetRandomnessAfter { after: Timestamp, job_id: String },

    // Withdraw the given amount to the withdrawal address
    Withdaw { amount: Coin },
    // Withdraw all available balance of this token to the withdrawal address
    WithdawAll { denom: String },
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
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get the config state
    #[returns(ConfigResponse)]
    Config {},
    /// Queries the local channel ID that is used to request beacons.
    /// This channel is unset when the proxy is instantiated and will be set when
    /// the channel is created. Once created, the value does not change anymore.
    #[returns(OracleChannelResponse)]
    OracleChannel {},
}

// We define a custom struct for each query response
pub type ConfigResponse = Config;

#[cw_serde]
pub struct OracleChannelResponse {
    pub channel: Option<String>,
}
