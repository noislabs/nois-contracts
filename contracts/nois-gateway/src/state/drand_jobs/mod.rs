mod drand_jobs1;
mod drand_jobs2;

use cosmwasm_std::{StdResult, Storage};
pub use drand_jobs1::Job;

// This is too inefficient for drand_jobs1 and only used in an informational query
pub use drand_jobs2::all_unprocessed_drand_jobs;

// New jobs always go to v2
pub use drand_jobs2::unprocessed_drand_jobs_enqueue;

pub fn unprocessed_drand_jobs_len(storage: &dyn Storage, round: u64) -> StdResult<u32> {
    Ok(drand_jobs1::unprocessed_drand_jobs_len(storage, round)?
        + drand_jobs2::unprocessed_drand_jobs_len(storage, round)?)
}

pub fn unprocessed_drand_jobs_dequeue(
    storage: &mut dyn Storage,
    round: u64,
) -> StdResult<Option<Job>> {
    if let Some(job_v1) = drand_jobs1::unprocessed_drand_jobs_dequeue(storage, round)? {
        return Ok(Some(job_v1));
    }
    if let Some(job_v2) = drand_jobs2::unprocessed_drand_jobs_dequeue(storage, round)? {
        return Ok(Some(job_v2));
    }
    Ok(None)
}
