use cosmwasm_std::{StdResult, Storage};
use cw_storage_plus::Map;

/// A map from drand rounds to number of jobs.
/// "pc" is short for processed count.
const PROCESSED_DRAND_JOBS_COUNT: Map<u64, u32> = Map::new("drand_jobs_pc");

/// Add an element to the processed drand jobs queue of this round
pub fn get_processed_drand_jobs(storage: &dyn Storage, round: u64) -> StdResult<u32> {
    let current = PROCESSED_DRAND_JOBS_COUNT
        .may_load(storage, round)?
        .unwrap_or(0);
    Ok(current)
}

/// Add an element to the processed drand jobs queue of this round
pub fn increment_processed_drand_jobs(storage: &mut dyn Storage, round: u64) -> StdResult<()> {
    let current = get_processed_drand_jobs(storage, round)?;
    PROCESSED_DRAND_JOBS_COUNT.save(storage, round, &(current + 1))?;
    Ok(())
}
