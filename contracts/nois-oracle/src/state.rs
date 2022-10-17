use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, HexBinary, StdResult, Storage, Timestamp, Uint128};
use cw_storage_plus::{Deque, Item, Map};

use crate::drand::time_of_round;

#[cw_serde]
pub struct Config {
    /// The lowest drand round this contracts accepts for verification and storage.
    pub min_round: u64,
    /// Bot incentive amount
    pub incentive_amount: Uint128,
    /// Bot incentive denom
    pub incentive_denom: String,
}

pub const CONFIG: Item<Config> = Item::new("config");

#[cw_serde]
pub struct VerifiedBeacon {
    pub verified: Timestamp,
    /// The sha256(signature) in lower case hex
    pub randomness: HexBinary,
}

/// Like VerifiedBeacon but plus round
#[cw_serde]
pub struct QueriedBeacon {
    pub round: u64,
    pub published: Timestamp,
    pub verified: Timestamp,
    /// The sha256(signature) in lower case hex
    pub randomness: HexBinary,
}

impl QueriedBeacon {
    pub fn make(beacon: VerifiedBeacon, round: u64) -> Self {
        Self {
            round,
            published: time_of_round(round),
            verified: beacon.verified,
            randomness: beacon.randomness,
        }
    }
}

// A map from round number to drand beacon
pub const BEACONS: Map<u64, VerifiedBeacon> = Map::new("beacons");

pub const BOTS: Map<&Addr, Bot> = Map::new("bots");

#[cw_serde]
pub struct StoredSubmission {
    pub time: Timestamp,
}

/// Stores the submission for an efficient (round, address) lookup
pub const SUBMISSIONS: Map<(u64, &Addr), StoredSubmission> = Map::new("submissions");

/// A map from (round, index) to bot address. This is used when
/// sorted submissions are needed.
///
/// The `index` values are 0-based. So the `n`th submission has index
/// n-1 here as well as in the response array in `SubmissionsResponse`.
pub const SUBMISSIONS_ORDER: Map<(u64, u32), Addr> = Map::new("submissions_order");

/// The bot type for the state. We don't need the address here
/// since this is stored in the storage key.
#[cw_serde]
pub struct Bot {
    pub moniker: String,
    /// Number of rounds added
    pub rounds_added: u64,
}

/// Like [`Bot`] but with address
#[cw_serde]
pub struct QueriedBot {
    pub moniker: String,
    pub address: Addr,
    /// Number of rounds added
    pub rounds_added: u64,
}

impl QueriedBot {
    pub fn make(beacon: Bot, address: Addr) -> Self {
        Self {
            address,
            moniker: beacon.moniker,
            rounds_added: beacon.rounds_added,
        }
    }
}

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

/// Add an element to the unprocessed drand jobs queue of this round
pub fn jobs_queue_enqueue(storage: &mut dyn Storage, round: u64, value: &Job) -> StdResult<()> {
    let prefix = format!("drand_jobs_{:0>10}", round);
    Deque::new(&prefix).push_back(storage, value)
}

/// Remove an element from the unprocessed drand jobs queue of this round
pub fn jobs_queue_dequeue(storage: &mut dyn Storage, round: u64) -> StdResult<Option<Job>> {
    let prefix = format!("drand_jobs_{:0>10}", round);
    Deque::new(&prefix).pop_front(storage)
}
