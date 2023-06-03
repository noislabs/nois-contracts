use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary, Coin, StdResult, Storage};
use cw_storage_plus::{Deque, Item, Map};

#[cw_serde]
pub struct Config {
    /// The address of the drand contract.
    /// As long as this is unset, noone can submit randomness.
    pub drand: Option<Addr>,
    /// Manager to set the price and drand address
    pub manager: Addr,
    /// The price to pay in order to register the randomness job
    pub price: Coin,
    /// The code ID of the payment contract to be instantatiated
    pub payment_code_id: u64,
    /// An amount the gateway sends to the payment contract during instantiation.
    /// Used for testing only to avoid draining the gateway's balance by opening channels.
    /// Use None or 0unois to disable.
    pub payment_initial_funds: Option<Coin>,
    /// Address of the Nois sink
    pub sink: Addr,
}

pub const CONFIG: Item<Config> = Item::new("config");

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

#[cw_serde]
pub struct Customer {
    /// The payment contract address
    pub payment: Addr,
    /// Number of beacons requested in total
    pub requested_beacons: u64,
}

/// A map from channel ID to customer information
pub const CUSTOMERS: Map<&str, Customer> = Map::new("customers");

/// A beacon request
#[cw_serde]
pub struct ProcessedRequest {
    pub origin: Binary,
    /// height and tx_index of the transaction in which this was added
    pub tx: (u64, Option<u32>),
    /// A RNG specific randomness source identifier, e.g. `drand:<network id>:<round>`
    pub source_id: String,
    /// This is true if the request was queued, i.e. the randomness is not yet available.
    /// It is false if the request is already available.
    pub queued: bool,
}

/// Add an element to the unprocessed drand jobs queue of this round
pub fn requests_add(
    storage: &mut dyn Storage,
    channel_id: &str,
    request: &ProcessedRequest,
) -> StdResult<()> {
    let prefix = requests_key(channel_id);
    Deque::new(&prefix).push_back(storage, request)
}

#[inline]
fn requests_key(channel_id: &str) -> String {
    format!("r_{channel_id}")
}
