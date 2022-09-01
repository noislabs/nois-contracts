use cosmwasm_std::{to_binary, Binary, StdResult};
use nois_ibc_protocol::Data;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// This needs no info. Owner of the contract is whoever signed the InstantiateMsg.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Get's the next randomness.
    GetNextRandomness {
        // A callback ID chosen by the caller
        callback_id: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {}

/// This hould be de/serialized under `Receive()` variant in a ExecuteMsg
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
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
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
enum ReceiverExecuteMsg {
    Receive(NoisCallbackMsg),
}
