use cosmwasm_schema::cw_serde;

use crate::Data;

#[cw_serde]
pub enum ProxyExecuteMsg {
    /// Get's the next randomness.
    GetNextRandomness {
        // A callback ID chosen by the caller
        callback_id: Option<String>,
    },
}

/// This hould be de/serialized under `Receive()` variant in a ExecuteMsg
#[cw_serde]
pub struct NoisCallbackMsg {
    /// The ID chosen by the caller in the `callback_id`
    pub id: String,
    pub randomness: Data,
}
