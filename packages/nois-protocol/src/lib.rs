mod checks;
mod ibc_msg;

use cosmwasm_std::IbcOrder;

pub use checks::{check_order, check_version, ChannelError};
pub use ibc_msg::{
    DeliverBeaconPacketAck, InPacket, InPacketAck, OutPacket, StdAck, WelcomePacketAck,
};

pub const IBC_APP_VERSION: &str = "nois-v6";
pub const APP_ORDER: IbcOrder = IbcOrder::Unordered;
// we use this for tests to ensure it is rejected
pub const BAD_APP_ORDER: IbcOrder = IbcOrder::Ordered;

// We currently have no way for dapps to process timeouts. If we wanted that,
// we had to introduce a different callback which then needs handling by the
// dapp developer. For now, let's assume all requests relayed to Nois and all
// deliveries are relayed to the consumer chain. In order to avoid unintended
// timeouts due to relayer downtime, we set the lifetime to 100 days.
pub const REQUEST_BEACON_PACKET_LIFETIME: u64 = 100 * 24 * 3600; // seconds
pub const DELIVER_BEACON_PACKET_LIFETIME: u64 = 100 * 24 * 3600; // seconds
pub const BEACON_PRICE_PACKET_LIFETIME: u64 = 100 * 24 * 3600; // seconds
pub const WELCOME_PACKET_LIFETIME: u64 = 100 * 24 * 3600; // seconds

/// Token transfers time out earlier than other messages to avoid locking funds for too long
pub const TRANSFER_PACKET_LIFETIME: u64 = 24 * 3600; // seconds
