use cosmwasm_schema::{cw_serde, QueryResponses};
use nois::ProxyExecuteMsg;

#[cw_serde]
pub struct InstantiateMsg {}

pub type ExecuteMsg = ProxyExecuteMsg;

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}
