use cosmwasm_schema::{cw_serde, QueryResponses};
use nois::{NoisCallbackMsg, ProxyExecuteMsg};

#[cw_serde]
pub struct InstantiateMsg {}

pub type ExecuteMsg = ProxyExecuteMsg;

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}

/// This is just a helper to properly serialize the above message.
/// The actual receiver should include this variant in the larger ExecuteMsg enum
#[cw_serde]
pub enum ReceiverExecuteMsg {
    Receive(NoisCallbackMsg),
}
