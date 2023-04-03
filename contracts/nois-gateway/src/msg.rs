use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin, HexBinary};

use crate::state::Config;

#[cw_serde]
pub struct InstantiateMsg {
    /// The price of a randomness.
    pub price: Coin,
    pub manager: String,
    pub payment_code_id: u64,
    /// An amount the gateway sends to the payment contract during instantiation.
    /// Use 0unois to diable.
    pub payment_initial_funds: Coin,
    /// Address of the Nois sink
    pub sink: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Add drand beacon
    AddVerifiedRound {
        round: u64,
        randomness: HexBinary,
        is_verifying_tx: bool,
    },
    /// Set the config
    SetConfig {
        manager: Option<String>,
        price: Option<Coin>,
        drand_addr: Option<String>,
        payment_code_id: Option<u64>,
        payment_initial_funds: Option<Coin>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get the config state
    #[returns(ConfigResponse)]
    Config {},
    /// Gets basic statistics about jobs in this drand round.
    #[returns(DrandJobStatsResponse)]
    DrandJobStats { round: u64 },
    #[returns(PaymentAddressResponse)]
    PaymentAddress { channel_id: String },
    #[returns(PaymentAddressesResponse)]
    PaymentAddresses {
        /// The channel ID after which to start
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

// We define a custom struct for each query response
pub type ConfigResponse = Config;

#[cw_serde]
pub struct DrandJobStatsResponse {
    pub round: u64,
    /// Number of unprocessed jobs
    pub unprocessed: u32,
    /// Number of processed jobs
    pub processed: u32,
}

#[cw_serde]
pub struct QueriedPaymentAddress {
    pub channel_id: String,
    /// The address of the payment contract
    pub address: Addr,
}

#[cw_serde]
pub struct PaymentAddressResponse {
    /// The address of the payment contract
    pub address: Option<Addr>,
}

#[cw_serde]
pub struct PaymentAddressesResponse {
    pub addresses: Vec<QueriedPaymentAddress>,
}
