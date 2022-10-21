use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Timestamp};
use cw_storage_plus::Item;

#[cw_serde]
pub struct Config {
    /// The prices of a randomness. List is to be interpreted as oneof,
    /// i.e. payment must be paid in one of those denominations.
    pub prices: Vec<Coin>,
    // The address to which withdrawals will be made
    pub withdrawal_address: Addr,
    pub test_mode: bool,
}

pub const CONFIG: Item<Config> = Item::new("config");

pub const ORACLE_CHANNEL: Item<String> = Item::new("oracle_channel");

/// We use this value to get publish times that are independent of the current clock
/// in test mode. We want the following rounds to be the result. To get there er use
/// a starting time of 1660940820000000000 - 1 nanoseconds and then increment by 30 seconds.
///
/// ```plain
/// Publish times (https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=65eea6af54e85243595743ba41f96f18)
/// Publish time of #2183660: 1660940820000000000
/// Publish time of #2183661: 1660940850000000000
/// Publish time of #2183662: 1660940880000000000
/// Publish time of #2183663: 1660940910000000000
/// Publish time of #2183664: 1660940940000000000
/// Publish time of #2183665: 1660940970000000000
/// Publish time of #2183666: 1660941000000000000
/// Publish time of #2183667: 1660941030000000000
/// Publish time of #2183668: 1660941060000000000
/// Publish time of #2183669: 1660941090000000000
/// Publish time of #2183670: 1660941120000000000
/// Publish time of #2183671: 1660941150000000000
/// Publish time of #2183672: 1660941180000000000
/// Publish time of #2183673: 1660941210000000000
/// Publish time of #2183674: 1660941240000000000
/// Publish time of #2183675: 1660941270000000000
/// Publish time of #2183676: 1660941300000000000
/// Publish time of #2183677: 1660941330000000000
/// Publish time of #2183678: 1660941360000000000
/// Publish time of #2183679: 1660941390000000000
/// Publish time of #2183680: 1660941420000000000
/// ```
pub const TEST_MODE_NEXT_AFTER: Item<Timestamp> = Item::new("test_mode_next_after");

pub const TEST_MODE_NEXT_AFTER_INIT: Timestamp = Timestamp::from_nanos(1660940820000000000 - 1);
pub const TEST_MODE_NEXT_AFTER_INCREMENT_SECONDS: u64 = 30;
