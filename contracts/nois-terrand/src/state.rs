use cw_storage_plus::Map;
use nois_ibc_protocol::Beacon;

// A map from round number to drand beacon
pub const ROUNDS: Map<u64, Beacon> = Map::new("rounds");
