use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Config {
    /// In test mode the next round calculation is detached from the clock.
    pub test_mode: bool,
}

pub const CONFIG: Item<Config> = Item::new("config");

pub const TERRAND_CHANNEL: Item<String> = Item::new("terrand_channel");

pub const TEST_MODE_NEXT_ROUND: Item<u64> = Item::new("test_mode_next_round");
