use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, HexBinary, Timestamp, Uint128};

/// This is the message we send over the IBC channel from nois-proxy to nois-gateway.
#[cw_serde]
#[non_exhaustive]
pub enum InPacket {
    RequestBeacon {
        /// Beacon publish time must be > `after`
        after: Timestamp,
        /// The origin data set by the proxy in a proxy specific format.
        origin: Binary,
    },
    /// Requests the current price per beacon. This can change over time and potentially
    /// change per channel ID.
    /// The proxy can pull the beacon price but should also expect price updates to get pushed.
    PullBeaconPrice {},
}

#[cw_serde]
#[non_exhaustive]
pub enum InPacketAck {
    /// Beacon already exists and this request can be processed immediately.
    RequestProcessed {
        /// A RNG specific randomness source identifier, e.g. `drand:<network id>:<round>`
        source_id: String,
    },
    /// Beacon does not yet exist. This request is queued for later.
    RequestQueued {
        /// A RNG specific randomness source identifier, e.g. `drand:<network id>:<round>`
        source_id: String,
    },
    /// The response of the PullBeaconPrice packet.
    PullBeaconPrice {
        /// The time of this price info. Since packages are not ordered, we use this to only save
        /// more recent price infos than we had before.
        timestamp: Timestamp,
        /// The amount in `denom`
        amount: Uint128,
        /// The denom on the Nois chain. This cannot be used directly here.
        denom: String,
    },
}

/// This is the message we send over the IBC channel from nois-gateway to nois-proxy.
#[cw_serde]
#[non_exhaustive]
pub enum OutPacket {
    DeliverBeacon {
        /// A RNG specific randomness source identifier, e.g. `drand:<network id>:<round>`.
        /// Please note that the source of randomness can change any time and users should not
        /// rely on the contents of this string.
        source_id: String,
        /// The point in time when the randomness was first published. This information is provided
        /// by the randomness provider. This is not the time when the randomness was processed on chain.
        published: Timestamp,
        /// The randomness. This is guaranteed to be 32 bytes long.
        randomness: HexBinary,
        /// The origin data set by the proxy in a proxy specific format.
        origin: Binary,
    },
    Welcome {
        /// Payment address on the Nois blockchain
        payment: String,
    },
    /// Proactively sends an update of the beacon price to the proxy.
    /// This is done together with the Welcome packet but can also happen any
    /// time later if pricing changes.
    PushBeaconPrice {
        /// The time of this price info. Since packages are not ordered, we use this to only save
        /// more recent price infos than we had before.
        timestamp: Timestamp,
        /// The amount in `denom`
        amount: Uint128,
        /// The denom on the Nois chain. This cannot be used directly here.
        denom: String,
    },
}

#[cw_serde]
#[non_exhaustive]
pub enum OutPacketAck {
    /// The ack the proxy must send when receiving a `OutPacket::DeliverBeacon`.
    ///
    /// This is a lighweight structure as the gateway does not do anything other than
    /// simple logging of the beacon delivery ack.
    DeliverBeacon {},
    /// The ack the proxy must send when receiving a `OutPacket::Welcome`.
    Welcome {},
    /// The ack the proxy must send when receiving a `OutPacket::PushBeaconPrice`.
    PushBeaconPrice {},
}
