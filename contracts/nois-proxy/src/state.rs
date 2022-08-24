use serde::{Deserialize, Serialize};

use cosmwasm_std::{IbcPacketAckMsg, Timestamp};
use cw_storage_plus::Item;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GetBeaconResponse {
    /// last block balance was updated (0 is never)
    pub last_update_time: Timestamp,
    pub response: IbcPacketAckMsg,
}

/// Stores the result of the latest query (coming from IBC)
/// for debugging purposes.
pub const LATEST_QUERY_RESULT: Item<GetBeaconResponse> = Item::new("latest_query_result");

pub const TERRAND_CHANNEL: Item<String> = Item::new("terrand_channel");
