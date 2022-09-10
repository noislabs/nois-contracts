mod checks;
mod ibc_msg;

use cosmwasm_std::IbcOrder;

pub use checks::{check_order, check_version, ChannelError};
pub use ibc_msg::{
    DeliverBeaconPacket, DeliverBeaconPacketAck, RequestBeaconPacket, RequestBeaconPacketAck,
    StdAck,
};

pub const IBC_APP_VERSION: &str = "nois-v2";
pub const APP_ORDER: IbcOrder = IbcOrder::Unordered;
// we use this for tests to ensure it is rejected
pub const BAD_APP_ORDER: IbcOrder = IbcOrder::Ordered;
