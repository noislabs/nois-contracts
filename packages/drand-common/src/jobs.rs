use cosmwasm_schema::cw_serde;

#[cw_serde]
pub struct DrandJobStatsResponse {
    pub round: u64,
    /// Number of unprocessed jobs
    pub unprocessed: u32,
    /// Number of processed jobs
    pub processed: u32,
}

#[cw_serde]
pub struct DrandJobStats {
    pub round: u64,
}
