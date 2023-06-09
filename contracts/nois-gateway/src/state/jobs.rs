use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, StdResult, Storage};
use cw_storage_plus::Deque;

#[cw_serde]
pub struct Job {
    /// A RNG specific randomness source identifier, e.g. `drand:<network id>:<round>`
    pub source_id: String,
    // The channel the job came from and we have to send the response to
    pub channel: String,
    pub origin: Binary,
}

#[inline]
fn unprocessed_drand_jobs_key(round: u64) -> String {
    // "up" for unprocessed
    // At a frequency of 1 beacon/second, 10 decimal places is sufficient for 300 years.
    format!("drand_jobs_up_{:0>10}", round)
}

/// Add an element to the unprocessed drand jobs queue of this round
pub fn unprocessed_drand_jobs_enqueue(
    storage: &mut dyn Storage,
    round: u64,
    value: &Job,
) -> StdResult<()> {
    let prefix = unprocessed_drand_jobs_key(round);
    Deque::new(&prefix).push_back(storage, value)
}

/// Remove an element from the unprocessed drand jobs queue of this round
pub fn unprocessed_drand_jobs_dequeue(
    storage: &mut dyn Storage,
    round: u64,
) -> StdResult<Option<Job>> {
    let prefix = unprocessed_drand_jobs_key(round);
    Deque::new(&prefix).pop_front(storage)
}

/// Gets the number of unprocessed drand jobs queue of this round
pub fn unprocessed_drand_jobs_len(storage: &dyn Storage, round: u64) -> StdResult<u32> {
    let prefix = unprocessed_drand_jobs_key(round);
    Deque::<Job>::new(&prefix).len(storage)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unprocessed_drand_jobs_key_works() {
        assert_eq!(unprocessed_drand_jobs_key(0), "drand_jobs_up_0000000000");
        assert_eq!(unprocessed_drand_jobs_key(1), "drand_jobs_up_0000000001");
        assert_eq!(unprocessed_drand_jobs_key(42), "drand_jobs_up_0000000042");
        assert_eq!(
            unprocessed_drand_jobs_key(2879178),
            "drand_jobs_up_0002879178"
        );
        assert_eq!(
            unprocessed_drand_jobs_key(9876543210),
            "drand_jobs_up_9876543210"
        );
    }
}
