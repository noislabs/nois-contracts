use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Timestamp, Uint128};
use cw_storage_plus::{Item, Map};

/// The denom information required to send a MsgTransfer.
/// Ideally we could just query the ICS-20 channel ID and did not have to store it,
/// but CosmWasm currently does not provide the query for it.
#[cw_serde]
pub struct IbcDenom {
    /// The ICS-20 channel ID of the NOIS token on the consummer chain
    pub ics20_channel: String,
    /// The ibc/* denom for the token
    pub denom: String,
}

/// Defines how the proxy handles payment of its randomness requests. This only affects
/// the proxy-Nois side. Users of the proxy always have to pay the amount set in `prices`.
#[cw_serde]
#[non_exhaustive]
pub enum OperationalMode {
    /// Someone fills the payment contract of the proxy on behalf of the proxy.
    /// This can happen onchain or offchain, automated or manually.
    Funded {},
    /// Proxy contract sends IBCed NOIS to the gateway for each beacon request.
    IbcPay {
        /// The denom of the IBCed unois token
        unois_denom: IbcDenom,
    },
}

#[cw_serde]
pub struct Config {
    /// The prices of a randomness. List is to be interpreted as oneof,
    /// i.e. payment must be paid in one of those denominations.
    /// If this list is empty, the user cannot pay. This can be used to put the
    /// contract out of service.
    pub prices: Vec<Coin>,
    /// Manager to set the config and withdraw funds
    pub manager: Option<Addr>,
    pub test_mode: bool,
    /// The amount of gas that the callback to the dapp can consume
    pub callback_gas_limit: u64,
    /// Address of the payment contract (on the other chain)
    pub payment: Option<String>,
    /// The amount of tokens the proxy sends for each randomness request to the Nois chain
    pub nois_beacon_price: Uint128,
    /// The time (on the Nois chain) the price info was created
    pub nois_beacon_price_updated: Timestamp,
    pub mode: OperationalMode,
    /// Enable whitelist of addresses allowed to get randomness.
    /// This is an Option for compatibility with older versions of the contract.
    /// If set to None it means disabled.
    /// From instances running version 0.13.5 onwards, the value is always set to Some(..).
    pub allowlist_enabled: Option<bool>,
    /// The minimal value for `after` when requesting a beacon.
    /// This aims to counter accidental misusage. Not all values in the allowed range are reasonable.
    /// This is an Option for compatibility with older versions of the contract that did not have the field.
    /// From instances running version 0.13.5 onwards, the value is always set to Some(..).
    pub min_after: Option<Timestamp>,
    /// The maximum value for `after` when requesting a beacon.
    /// This aims to counter accidental misusage. Not all values in the allowed range are reasonable.
    /// This is an Option for compatibility with older versions of the contract that did not have the field.
    /// From instances running version 0.13.5 onwards, the value is always set to Some(..).
    pub max_after: Option<Timestamp>,
}

pub const CONFIG: Item<Config> = Item::new("config");

/// List of addresses allowed to get randomness if allowlist enabled. To decide
/// if an address is allowed, we consider only whether the address is present as
/// a key. The u8 value itself is a dummy value.
pub const ALLOWLIST: Map<&Addr, u8> = Map::new("allowlist");

/// Dummy value. Don't rely on the value but just check existence.
pub const ALLOWLIST_MARKER: u8 = 1;

/// Channel to the nois-gateway contract on the Nois chain
pub const GATEWAY_CHANNEL: Item<String> = Item::new("gateway_channel");

/// We use this value to get publish times that are independent of the current clock
/// in test mode. We want the following rounds to be the result. To get there we use
/// a starting time of 1677687597000000000 - 1 nanoseconds and then increment by 30 seconds.
///
/// ```plain
/// Publish times (https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=4d0d6d84cdb9b48260594e3b74aa86ae)
/// Publish time of #800: 1677687597000000000
/// Publish time of #801: 1677687600000000000
/// Publish time of #802: 1677687603000000000
/// Publish time of #803: 1677687606000000000
/// Publish time of #804: 1677687609000000000
/// Publish time of #805: 1677687612000000000
/// Publish time of #806: 1677687615000000000
/// Publish time of #807: 1677687618000000000
/// Publish time of #808: 1677687621000000000
/// Publish time of #809: 1677687624000000000
/// Publish time of #810: 1677687627000000000
/// Publish time of #811: 1677687630000000000
/// Publish time of #812: 1677687633000000000
/// Publish time of #813: 1677687636000000000
/// Publish time of #814: 1677687639000000000
/// Publish time of #815: 1677687642000000000
/// Publish time of #816: 1677687645000000000
/// Publish time of #817: 1677687648000000000
/// Publish time of #818: 1677687651000000000
/// Publish time of #819: 1677687654000000000
/// Publish time of #820: 1677687657000000000
/// Publish time of #821: 1677687660000000000
/// Publish time of #822: 1677687663000000000
/// Publish time of #823: 1677687666000000000
/// Publish time of #824: 1677687669000000000
/// Publish time of #825: 1677687672000000000
/// Publish time of #826: 1677687675000000000
/// Publish time of #827: 1677687678000000000
/// Publish time of #828: 1677687681000000000
/// Publish time of #829: 1677687684000000000
/// Publish time of #830: 1677687687000000000
/// Publish time of #831: 1677687690000000000
/// Publish time of #832: 1677687693000000000
/// Publish time of #833: 1677687696000000000
/// Publish time of #834: 1677687699000000000
/// Publish time of #835: 1677687702000000000
/// Publish time of #836: 1677687705000000000
/// Publish time of #837: 1677687708000000000
/// Publish time of #838: 1677687711000000000
/// Publish time of #839: 1677687714000000000
/// Publish time of #840: 1677687717000000000
/// Publish time of #841: 1677687720000000000
/// Publish time of #842: 1677687723000000000
/// Publish time of #843: 1677687726000000000
/// Publish time of #844: 1677687729000000000
/// Publish time of #845: 1677687732000000000
/// Publish time of #846: 1677687735000000000
/// Publish time of #847: 1677687738000000000
/// Publish time of #848: 1677687741000000000
/// Publish time of #849: 1677687744000000000
/// Publish time of #850: 1677687747000000000
/// Publish time of #851: 1677687750000000000
/// Publish time of #852: 1677687753000000000
/// Publish time of #853: 1677687756000000000
/// Publish time of #854: 1677687759000000000
/// Publish time of #855: 1677687762000000000
/// Publish time of #856: 1677687765000000000
/// Publish time of #857: 1677687768000000000
/// Publish time of #858: 1677687771000000000
/// Publish time of #859: 1677687774000000000
/// Publish time of #860: 1677687777000000000
/// Publish time of #861: 1677687780000000000
/// Publish time of #862: 1677687783000000000
/// Publish time of #863: 1677687786000000000
/// Publish time of #864: 1677687789000000000
/// Publish time of #865: 1677687792000000000
/// Publish time of #866: 1677687795000000000
/// Publish time of #867: 1677687798000000000
/// Publish time of #868: 1677687801000000000
/// Publish time of #869: 1677687804000000000
/// Publish time of #870: 1677687807000000000
/// Publish time of #871: 1677687810000000000
/// Publish time of #872: 1677687813000000000
/// Publish time of #873: 1677687816000000000
/// Publish time of #874: 1677687819000000000
/// Publish time of #875: 1677687822000000000
/// Publish time of #876: 1677687825000000000
/// Publish time of #877: 1677687828000000000
/// Publish time of #878: 1677687831000000000
/// Publish time of #879: 1677687834000000000
/// Publish time of #880: 1677687837000000000
/// Publish time of #881: 1677687840000000000
/// Publish time of #882: 1677687843000000000
/// Publish time of #883: 1677687846000000000
/// Publish time of #884: 1677687849000000000
/// Publish time of #885: 1677687852000000000
/// Publish time of #886: 1677687855000000000
/// Publish time of #887: 1677687858000000000
/// Publish time of #888: 1677687861000000000
/// Publish time of #889: 1677687864000000000
/// Publish time of #890: 1677687867000000000
/// Publish time of #891: 1677687870000000000
/// Publish time of #892: 1677687873000000000
/// Publish time of #893: 1677687876000000000
/// Publish time of #894: 1677687879000000000
/// Publish time of #895: 1677687882000000000
/// Publish time of #896: 1677687885000000000
/// Publish time of #897: 1677687888000000000
/// Publish time of #898: 1677687891000000000
/// Publish time of #899: 1677687894000000000
/// Publish time of #900: 1677687897000000000
/// ```
pub const TEST_MODE_NEXT_AFTER: Item<Timestamp> = Item::new("test_mode_next_after");

pub const TEST_MODE_NEXT_AFTER_INIT: Timestamp = Timestamp::from_nanos(1677687597000000000 - 1);
pub const TEST_MODE_NEXT_AFTER_INCREMENT_SECONDS: u64 = 30;
