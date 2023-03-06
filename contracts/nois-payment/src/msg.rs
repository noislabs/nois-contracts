use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;

#[cw_serde]
pub struct InstantiateMsg {
    pub nois_sink: String,
    pub nois_com_pool_addr: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Burns the tokens that are sent as `funds` with this message
    Pay {
        burn: Uint128,
        community_pool: Uint128,
        relayer: (String, Uint128),
    },
}

#[cw_serde]
pub enum NoisSinkExecuteMsg {
    /// Burns the tokens that are sent as `funds` with this message
    Burn {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}
