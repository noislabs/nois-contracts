use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use nois_proxy::NoisCallbackMsg;

/// This needs no info. Owner of the contract is whoever signed the InstantiateMsg.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InstantiateMsg {
    /// Address of the Nois proxy contract
    pub nois_proxy: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    EstimatePi {
        round: u64,
        /// An ID for this job which allows for gathering the results.
        job_id: String,
    },
    Receive(NoisCallbackMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // Get all results
    Results {},
    // Get latest result
    LatestResult {},
}
