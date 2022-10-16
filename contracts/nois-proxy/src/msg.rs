use cosmwasm_schema::{cw_serde, QueryResponses};
use nois::ProxyExecuteMsg;

#[cw_serde]
pub struct InstantiateMsg {
    /// In test mode the min publish time calculation is detached from the clock.
    pub test_mode: bool,
}

pub type ExecuteMsg = ProxyExecuteMsg;

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Queries the local channel ID that is used to request beacons.
    /// This channel is unset when the proxy is instantiated and will be set when
    /// the channel is created. Once created, the value does not change anymore.
    #[returns(OracleChannelResponse)]
    OracleChannel {},
}

#[cw_serde]
pub struct OracleChannelResponse {
    pub channel: Option<String>,
}
