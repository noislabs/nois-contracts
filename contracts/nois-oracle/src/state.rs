use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, StdResult, Storage};
use cw_storage_plus::{Deque, Item, Map};

#[cw_serde]
pub struct Config {
    /// The address of the drand contract
    pub drand: Option<Addr>,
}

pub const CONFIG: Item<Config> = Item::new("config");

#[cw_serde]
pub struct Job {
    /// A RNG specific randomness source identifier, e.g. `drand:<network id>:<round>`
    pub source_id: String,
    // The channel the job came from and we have to send the response to
    pub channel: String,
    // contract address on the app chain
    pub sender: String,
    /// A job ID assigned by the caller
    pub job_id: String,
}

#[inline]
fn unprocessed_jobs_key(round: u64) -> String {
    // "up" for unprocessed
    format!("jobs_up_{:0>10}", round)
}

/// Add an element to the unprocessed drand jobs queue of this round
pub fn unprocessed_jobs_enqueue(
    storage: &mut dyn Storage,
    round: u64,
    value: &Job,
) -> StdResult<()> {
    let prefix = unprocessed_jobs_key(round);
    Deque::new(&prefix).push_back(storage, value)
}

/// Remove an element from the unprocessed drand jobs queue of this round
pub fn unprocessed_jobs_dequeue(storage: &mut dyn Storage, round: u64) -> StdResult<Option<Job>> {
    let prefix = unprocessed_jobs_key(round);
    Deque::new(&prefix).pop_front(storage)
}

/// Gets the number of unprocessed drand jobs queue of this round
pub fn unprocessed_jobs_len(storage: &dyn Storage, round: u64) -> StdResult<u32> {
    let prefix = unprocessed_jobs_key(round);
    Deque::<Job>::new(&prefix).len(storage)
}

// "pc" for processed count
const PROCESSED_JOBS_COUNT: Map<u64, u32> = Map::new("jobs_pc");

/// Add an element to the processed drand jobs queue of this round
pub fn get_processed_jobs(storage: &dyn Storage, round: u64) -> StdResult<u32> {
    let current = PROCESSED_JOBS_COUNT.may_load(storage, round)?.unwrap_or(0);
    Ok(current)
}

/// Add an element to the processed drand jobs queue of this round
pub fn increment_processed_jobs(storage: &mut dyn Storage, round: u64) -> StdResult<()> {
    let current = get_processed_jobs(storage, round)?;
    PROCESSED_JOBS_COUNT.save(storage, round, &(current + 1))?;
    Ok(())
}
