use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Coin;

use crate::state::Config;

#[cw_serde]
pub struct InstantiateMsg {
    /// Address of the Nois sink
    pub sink: String,
    /// Address of the Nois community pool
    pub community_pool: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Pays by distributing the funds according to what has been instructed by the gateway
    Pay {
        burn: Coin,
        community_pool: Coin,
        relayer: (String, Coin),
    },
}

#[cw_serde]
pub enum NoisSinkExecuteMsg {
    /// Burns the tokens that are sent as `funds` with this message
    Burn {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get the config state
    #[returns(ConfigResponse)]
    Config {},
}

// We define a custom struct for each query response
pub type ConfigResponse = Config;
