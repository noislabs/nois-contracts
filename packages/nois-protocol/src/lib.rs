mod checks;
mod ibc_msg;
mod never;

use cosmwasm_std::IbcOrder;

pub use checks::{check_order, check_version, ChannelError};
pub use ibc_msg::{
    DeliverBeaconPacket, DeliverBeaconPacketAck, RequestBeaconPacket, RequestBeaconPacketAck,
    StdAck,
};
pub use never::Never;

pub const IBC_APP_VERSION: &str = "nois-v3";
pub const APP_ORDER: IbcOrder = IbcOrder::Unordered;
// we use this for tests to ensure it is rejected
pub const BAD_APP_ORDER: IbcOrder = IbcOrder::Ordered;
