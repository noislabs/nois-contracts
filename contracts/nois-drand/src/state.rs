use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, HexBinary, Timestamp, Uint128};
use cw_storage_plus::{Item, Map};

use drand_common::time_of_round;

/// Top level storage key. Values must not conflict.
/// Each key is only one byte long to ensure we use the smallest possible storage keys.
#[repr(u8)]
pub enum TopKey {
    /// Incentivized rounds
    Incentivized = b'i',
}

impl TopKey {
    const fn as_str(&self) -> &str {
        let data = unsafe { std::mem::transmute::<&Self, &[u8; 1]>(self) };
        match std::str::from_utf8(data) {
            Ok(a) => a,
            Err(_) => panic!("Non-utf8 enum value found. Use a-z, A-Z and 0-9"),
        }
    }
}

#[cw_serde]
pub struct Config {
    /// manager for bot addr de/allowlist
    pub manager: Addr,
    /// The address of the nois-gateway contract
    pub gateway: Option<Addr>,
    /// The lowest drand round this contracts accepts for verification and storage.
    pub min_round: u64,
    /// How much unois is given per incentive point
    pub incentive_point_price: Uint128,
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
/// An entry of this map looks like round_number =>  {verified_time, randomness}
pub const BEACONS: Map<u64, VerifiedBeacon> = Map::new("beacons");

pub const BOTS: Map<&Addr, Bot> = Map::new("bots");
pub const ALLOWLIST: Map<&Addr, ()> = Map::new("allowlist");

#[cw_serde]
pub struct StoredSubmission {
    /// The position which this submission was made within one round.
    /// This is used for sorting in `query_submissions`.
    pub pos: u16,
    /// Submission time (block time)
    pub time: Timestamp,
    /// Submission block height
    pub height: u64,
    /// Submission tx index
    pub tx_index: Option<u32>,
}

/// Stores the submission for an efficient (round, address) lookup
/// An entry of this map looks like (round, drand_bot_addr) =>  time
pub const SUBMISSIONS: Map<(u64, &Addr), StoredSubmission> = Map::new("submissions");

/// The number of submissions done for each round
pub const SUBMISSIONS_COUNT: Map<u64, u16> = Map::new("counts");

/// Dummy value. Don't rely on the value but just check existence.
pub const INCENTIVIZED_BY_GATEWAY_MARKER: u8 = 1;

/// A map from round number to marker value
pub const INCENTIVIZED_BY_GATEWAY: Map<u64, u8> = Map::new(TopKey::Incentivized.as_str());

/// The bot type for the state. We don't need the address here
/// since this is stored in the storage key.
#[cw_serde]
pub struct Bot {
    pub moniker: String,
    /// Number of rounds added.
    /// All valid round submissions are counted independently of the reward.
    pub rounds_added: u64,
    /// The total number of reward points collected.
    pub reward_points: u64,
}

/// Like [`Bot`] but with address
#[cw_serde]
pub struct QueriedBot {
    pub moniker: String,
    pub address: Addr,
    /// Number of rounds added.
    /// All valid round submissions are counted independently of the reward.
    pub rounds_added: u64,
    /// The total number of reward points collected.
    pub reward_points: u64,
}

impl QueriedBot {
    pub fn make(bot: Bot, address: Addr) -> Self {
        Self {
            address,
            moniker: bot.moniker,
            rounds_added: bot.rounds_added,
            reward_points: bot.reward_points,
        }
    }
}
