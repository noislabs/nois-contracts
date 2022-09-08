use cosmwasm_schema::{cw_serde, QueryResponses};

use nois_proxy::NoisCallbackMsg;

#[cw_serde]
pub struct InstantiateMsg {
    /// Address of the Nois proxy contract
    pub nois_proxy: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    EstimatePi {
        /// An ID for this job which allows for gathering the results.
        job_id: String,
    },
    Receive(NoisCallbackMsg),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // Get all results
    #[returns(Vec<String>)]
    Results {},
    // Get a result
    #[returns(Option<cosmwasm_std::Decimal>)]
    Result { job_id: String },
}
