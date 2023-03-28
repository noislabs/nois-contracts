use cosmwasm_schema::cw_serde;
use cosmwasm_std::{from_slice, to_binary, Binary, HexBinary, Timestamp};
use serde::de::DeserializeOwned;
use serde::Serialize;

/// This is the message we send over the IBC channel from nois-proxy to nois-gateway
#[cw_serde]
pub struct RequestBeaconPacket {
    /// Beacon publish time must be > `after`
    pub after: Timestamp,
    /// The origin data set by the proxy in a proxy specific format.
    pub origin: Binary,
}

#[cw_serde]
pub enum RequestBeaconPacketAck {
    /// Beacon already exists and this request can be processed immediately.
    Processed {
        /// A RNG specific randomness source identifier, e.g. `drand:<network id>:<round>`
        source_id: String,
    },
    /// Beacon does not yet exist. This request is queued for later.
    Queued {
        /// A RNG specific randomness source identifier, e.g. `drand:<network id>:<round>`
        source_id: String,
    },
}

/// This is the message we send over the IBC channel from nois-gateway to nois-proxy.
#[cw_serde]
#[non_exhaustive]
pub enum OutPacket {
    DeliverBeacon {
        /// A RNG specific randomness source identifier, e.g. `drand:<network id>:<round>`
        source_id: String,
        randomness: HexBinary,
        /// The origin data set by the proxy in a proxy specific format.
        origin: Binary,
    },
    Welcome {
        /// Payment address on the Nois blockchain
        payment: String,
    },
}

/// The ack the proxy must send when receiving a `OutPacket::DeliverBeacon`.
///
/// This is a lighweight structure as the gateway does not do anything other than
/// simple logging of the beacon delivery ack.
#[non_exhaustive]
#[cw_serde]
#[derive(Default)]
pub struct DeliverBeaconPacketAck {}

/// The ack the proxy must send when receiving a `OutPacket::Welcome`.
#[non_exhaustive]
#[cw_serde]
#[derive(Default)]
pub struct WelcomePacketAck {}

/// This is a generic ICS acknowledgement format.
/// Proto defined here: https://github.com/cosmos/cosmos-sdk/blob/v0.42.0/proto/ibc/core/channel/v1/channel.proto#L141-L147
/// If ibc_receive_packet returns Err(), then x/wasm runtime will rollback the state and return an error message in this format
#[cw_serde]
pub enum StdAck {
    Result(Binary),
    Error(String),
}

impl StdAck {
    /// Creates a result ack
    pub fn success(result: impl Serialize) -> Self {
        let serialized = to_binary(&result).unwrap();
        StdAck::Result(serialized)
    }

    /// Creates an error ack
    pub fn error(err: impl Into<String>) -> Self {
        StdAck::Error(err.into())
    }

    pub fn unwrap(self) -> Binary {
        match self {
            StdAck::Result(data) => data,
            StdAck::Error(err) => panic!("{}", err),
        }
    }

    pub fn unwrap_into<T: DeserializeOwned>(self) -> T {
        from_slice(&self.unwrap()).unwrap()
    }

    pub fn unwrap_err(self) -> String {
        match self {
            StdAck::Result(_) => panic!("not an error"),
            StdAck::Error(err) => err,
        }
    }
}

impl From<StdAck> for Binary {
    fn from(original: StdAck) -> Binary {
        // pretty sure this cannot fail
        to_binary(&original).unwrap()
    }
}
