pub use crate::{APP_ORDER, IBC_APP_VERSION};
use cosmwasm_std::IbcOrder;

use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ChannelError {
    #[error("Only supports unordered channels")]
    InvalidChannelOrder,

    #[error("Counterparty version must be '{0}'")]
    InvalidChannelVersion(&'static str),
}

pub fn check_order(order: &IbcOrder) -> Result<(), ChannelError> {
    if order != &APP_ORDER {
        Err(ChannelError::InvalidChannelOrder)
    } else {
        Ok(())
    }
}

pub fn check_version(version: &str) -> Result<(), ChannelError> {
    if version != IBC_APP_VERSION {
        Err(ChannelError::InvalidChannelVersion(IBC_APP_VERSION))
    } else {
        Ok(())
    }
}
