use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_binary, Binary, StdResult};
use nois_protocol::Data;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    /// Get's the next randomness.
    GetNextRandomness {
        // A callback ID chosen by the caller
        callback_id: Option<String>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}

/// This hould be de/serialized under `Receive()` variant in a ExecuteMsg
#[cw_serde]
pub struct NoisCallbackMsg {
    /// The ID chosen by the caller in the `callback_id`
    pub id: String,
    pub randomness: Data,
}

impl NoisCallbackMsg {
    /// Serializes the message into a Receive case
    pub fn into_wrapped_binary(self) -> StdResult<Binary> {
        let msg = ReceiverExecuteMsg::Receive(self);
        to_binary(&msg)
    }
}

/// This is just a helper to properly serialize the above message.
/// The actual receiver should include this variant in the larger ExecuteMsg enum
#[cw_serde]
enum ReceiverExecuteMsg {
    Receive(NoisCallbackMsg),
}
