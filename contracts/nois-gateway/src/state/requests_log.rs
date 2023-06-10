use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, StdResult, Storage};
use cw_storage_plus::Deque;

/// An accepted beacon request that is logged
#[cw_serde]
pub struct RequestLogEntry {
    /// The dapp specific origin value. This is not parsed here but just stored.
    pub origin: Binary,
    /// height and tx_index of the transaction in which this was added
    pub tx: (u64, Option<u32>),
    /// A RNG specific randomness source identifier, e.g. `drand:<network id>:<round>`
    pub source_id: String,
    /// This is true if the request was queued, i.e. the randomness is not yet available.
    /// It is false if the request is already available.
    pub queued: bool,
}

/// Add an element to the requests log for this customer.
/// An element cannot easily be updated since you cannot efficiently find one
/// specific item in the list.
/// Request logs may be cleared in the future.
pub fn requests_log_add(
    storage: &mut dyn Storage,
    channel_id: &str,
    request_log_entry: &RequestLogEntry,
) -> StdResult<()> {
    let prefix = requests_log_key(channel_id);
    Deque::new(&prefix).push_back(storage, request_log_entry)
}

pub fn requests_log_asc(
    storage: &dyn Storage,
    channel_id: &str,
    offset: usize,
    limit: usize,
) -> StdResult<Vec<RequestLogEntry>> {
    let prefix = requests_log_key(channel_id);
    Deque::new(&prefix)
        .iter(storage)?
        .skip(offset)
        .take(limit)
        .collect()
}

pub fn requests_log_desc(
    storage: &dyn Storage,
    channel_id: &str,
    offset: usize,
    limit: usize,
) -> StdResult<Vec<RequestLogEntry>> {
    let prefix = requests_log_key(channel_id);
    Deque::new(&prefix)
        .iter(storage)?
        .rev()
        .skip(offset)
        .take(limit)
        .collect()
}

#[inline]
fn requests_log_key(channel_id: &str) -> String {
    format!("rl_{channel_id}")
}
