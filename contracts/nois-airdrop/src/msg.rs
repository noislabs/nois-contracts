use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;

#[cw_serde]
pub struct InstantiateMsg {
    /// manager if none set to info.sender.
    pub manager: Option<String>,
    /// For unclaimed tokens
    pub withdrawal_address: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    UpdateConfig {
        /// Newmanager if non sent, contract gets locked. Recipients can receive airdrops
        /// but manager cannot register new stages.
        new_manager: Option<String>,
    },
    RegisterMerkleRoot {
        /// MerkleRoot is hex-encoded merkle root.
        merkle_root: String,
    },
    /// Claim does not check if contract has enough funds, manager must ensure it.
    Claim {
        amount: Uint128,
        /// Proof is hex-encoded merkle proof.
        proof: Vec<String>,
    },
    // Withdraw all available balance of this token to the withdrawal address
    WithdawAll {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},
    #[returns(MerkleRootResponse)]
    MerkleRoot {},
    #[returns(IsClaimedResponse)]
    IsClaimed { address: String },
}

#[cw_serde]
pub struct ConfigResponse {
    pub manager: Option<String>,
}

#[cw_serde]
pub struct MerkleRootResponse {
    /// MerkleRoot is hex-encoded merkle root.
    pub merkle_root: String,
}

#[cw_serde]
pub struct IsClaimedResponse {
    pub is_claimed: bool,
}

#[cw_serde]
pub struct MigrateMsg {}
