use cosmwasm_schema::{cw_serde, QueryResponses};

use nois::NoisCallback;

use crate::state::{JobLifecycleDelivery, JobLifecycleRequest};

#[cw_serde]
pub struct InstantiateMsg {
    pub nois_proxy: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    // job_id for this job which allows for gathering the results.
    RollDice { job_id: String },
    NoisReceive { callback: NoisCallback },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // GetCount returns the current count as a json-encoded number
    #[returns(Option<u8>)]
    Outcome { job_id: String },
    #[returns(Vec<String>)]
    GetHistoryOfRounds {},
    #[returns(Option<JobLifecycleRequest>)]
    GetRequest { job_id: String },
    #[returns(Option<JobLifecycleDelivery>)]
    GetDelivery { job_id: String },
}
