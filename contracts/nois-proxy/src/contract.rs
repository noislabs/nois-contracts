#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Deps, DepsMut, Env, Ibc3ChannelOpenResponse, IbcBasicResponse,
    IbcChannelCloseMsg, IbcChannelConnectMsg, IbcChannelOpenMsg, IbcMsg, IbcPacketAckMsg,
    IbcPacketReceiveMsg, IbcPacketTimeoutMsg, IbcReceiveResponse, MessageInfo, QueryResponse,
    Response, StdResult, Storage, SubMsg, Timestamp, WasmMsg,
};
use nois_ibc_protocol::{
    check_order, check_version, DeliverBeaconPacket, DeliverBeaconPacketAck, RequestBeaconPacket,
    RequestBeaconPacketAck, StdAck,
};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Config, CONFIG, TERRAND_CHANNEL, TEST_MODE_NEXT_ROUND};
use crate::NoisCallbackMsg;

// TODO: make configurable?
/// packets live one hour
pub const PACKET_LIFETIME: u64 = 60 * 60;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let config = Config {
        test_mode: msg.test_mode,
    };
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::GetNextRandomness { callback_id } => {
            execute_get_next_randomness(deps, env, info, callback_id)
        }
    }
}

pub fn execute_get_next_randomness(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    callback_id: Option<String>,
) -> StdResult<Response> {
    let sender = info.sender.into();
    let Config { test_mode } = CONFIG.load(deps.storage)?;
    let mode = if test_mode {
        NextRoundMode::Test
    } else {
        NextRoundMode::Time {
            base: env.block.time,
        }
    };
    let round = next_round(deps.storage, mode)?;
    let packet = RequestBeaconPacket {
        round,
        sender,
        callback_id,
    };
    let channel_id = TERRAND_CHANNEL.load(deps.storage)?;
    let msg = IbcMsg::SendPacket {
        channel_id,
        data: to_binary(&packet)?,
        timeout: env.block.time.plus_seconds(PACKET_LIFETIME).into(),
    };

    let res = Response::new()
        .add_message(msg)
        .add_attribute("action", "execute_get_next_randomness");
    Ok(res)
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum NextRoundMode {
    Test,
    Time { base: Timestamp },
}

const DRAND_GENESIS: Timestamp = Timestamp::from_seconds(1595431050);
const DRAND_ROUND_LENGTH: u64 = 30_000_000_000; // in nanoseconds

/// Calculates the next round in the future, i.e. publish time > base time.
fn next_round(storage: &mut dyn Storage, mode: NextRoundMode) -> StdResult<u64> {
    match mode {
        NextRoundMode::Test => {
            let next = TEST_MODE_NEXT_ROUND.may_load(storage)?.unwrap_or(2183660);
            TEST_MODE_NEXT_ROUND.save(storage, &(next + 1))?;
            Ok(next)
        }
        NextRoundMode::Time { base } => {
            // Losely ported from https://github.com/drand/drand/blob/eb36ba81e3f28c966f95bcd602f60e7ff8ef4c35/chain/time.go#L49-L63
            if base < DRAND_GENESIS {
                Ok(1)
            } else {
                let from_genesis = base.nanos() - DRAND_GENESIS.nanos();
                let next_round = (from_genesis / DRAND_ROUND_LENGTH) + 1;
                Ok(next_round + 1)
            }
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<QueryResponse> {
    match msg {}
}

#[cfg_attr(not(feature = "library"), entry_point)]
/// enforces ordering and versioing constraints
pub fn ibc_channel_open(
    _deps: DepsMut,
    _env: Env,
    msg: IbcChannelOpenMsg,
) -> Result<Option<Ibc3ChannelOpenResponse>, ContractError> {
    let channel = msg.channel();
    check_order(&channel.order)?;
    check_version(&channel.version)?;
    if let Some(counter_version) = msg.counterparty_version() {
        check_version(counter_version)?;
    }

    Ok(None)
}

#[cfg_attr(not(feature = "library"), entry_point)]
/// Once established we store the channel ID to look up
/// the destination address later.
pub fn ibc_channel_connect(
    deps: DepsMut,
    _env: Env,
    msg: IbcChannelConnectMsg,
) -> StdResult<IbcBasicResponse> {
    let channel = msg.channel();
    let channel_id = &channel.endpoint.channel_id;

    TERRAND_CHANNEL.save(deps.storage, channel_id)?;

    Ok(IbcBasicResponse::new()
        .add_attribute("action", "ibc_connect")
        .add_attribute("channel_id", channel_id))
}

#[cfg_attr(not(feature = "library"), entry_point)]
/// On closed channel, simply delete the account from our local store
pub fn ibc_channel_close(
    _deps: DepsMut,
    _env: Env,
    msg: IbcChannelCloseMsg,
) -> StdResult<IbcBasicResponse> {
    let channel = msg.channel();

    // remove the channel
    let channel_id = &channel.endpoint.channel_id;

    Ok(IbcBasicResponse::new()
        .add_attribute("action", "ibc_close")
        .add_attribute("channel_id", channel_id))
}

#[cfg_attr(not(feature = "library"), entry_point)]
/// never should be called as the other side never sends packets
pub fn ibc_packet_receive(
    _deps: DepsMut,
    _env: Env,
    packet: IbcPacketReceiveMsg,
) -> StdResult<IbcReceiveResponse> {
    let DeliverBeaconPacket {
        round: _,
        randomness,
        sender,
        callback_id,
    } = from_binary(&packet.packet.data)?;

    let acknowledgement = StdAck::success(&DeliverBeaconPacketAck {});

    match callback_id {
        Some(callback_id) => {
            // Send IBC packet ack message to another contract
            let msg = SubMsg::new(WasmMsg::Execute {
                contract_addr: sender,
                msg: NoisCallbackMsg {
                    id: callback_id.clone(),
                    randomness,
                }
                .into_wrapped_binary()?,
                funds: vec![],
            })
            .with_gas_limit(2_000_000);
            Ok(IbcReceiveResponse::new()
                .set_ack(acknowledgement)
                .add_attribute("action", "acknowledge_ibc_query")
                .add_attribute("callback_id", callback_id)
                .add_submessage(msg))
        }
        None => Ok(IbcReceiveResponse::new()
            .set_ack(acknowledgement)
            .add_attribute("action", "acknowledge_ibc_query")),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_packet_ack(
    _deps: DepsMut,
    _env: Env,
    msg: IbcPacketAckMsg,
) -> Result<IbcBasicResponse, ContractError> {
    let ack: StdAck = from_binary(&msg.acknowledgement.data)?;
    match ack {
        StdAck::Result(data) => {
            let _response: RequestBeaconPacketAck = from_binary(&data)?;
            // alright
            Ok(IbcBasicResponse::new().add_attribute("action", "RequestBeaconPacketAck"))
        }
        StdAck::Error(err) => Err(ContractError::ForeignError { err }),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
/// we just ignore these now. shall we store some info?
pub fn ibc_packet_timeout(
    _deps: DepsMut,
    _env: Env,
    _msg: IbcPacketTimeoutMsg,
) -> StdResult<IbcBasicResponse> {
    Ok(IbcBasicResponse::new().add_attribute("action", "ibc_packet_timeout"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{
        testing::{
            mock_dependencies, mock_env, mock_ibc_channel_open_try, mock_info, MockApi,
            MockQuerier, MockStorage,
        },
        OwnedDeps,
    };
    use nois_ibc_protocol::{APP_ORDER, BAD_APP_ORDER, IBC_APP_VERSION};

    const CREATOR: &str = "creator";

    fn setup() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg { test_mode: true };
        let info = mock_info(CREATOR, &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        deps
    }

    #[test]
    fn instantiate_works() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg { test_mode: true };
        let info = mock_info(CREATOR, &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn enforce_version_in_handshake() {
        let mut deps = setup();

        let wrong_order = mock_ibc_channel_open_try("channel-12", BAD_APP_ORDER, IBC_APP_VERSION);
        ibc_channel_open(deps.as_mut(), mock_env(), wrong_order).unwrap_err();

        let wrong_version = mock_ibc_channel_open_try("channel-12", APP_ORDER, "other version");
        ibc_channel_open(deps.as_mut(), mock_env(), wrong_version).unwrap_err();

        let valid_handshake = mock_ibc_channel_open_try("channel-12", APP_ORDER, IBC_APP_VERSION);
        ibc_channel_open(deps.as_mut(), mock_env(), valid_handshake).unwrap();
    }

    #[test]
    fn next_round_works_for_test_mode() {
        let mut deps = mock_dependencies();
        let round = next_round(&mut deps.storage, NextRoundMode::Test).unwrap();
        assert_eq!(round, 2183660);
        let round = next_round(&mut deps.storage, NextRoundMode::Test).unwrap();
        assert_eq!(round, 2183661);
        let round = next_round(&mut deps.storage, NextRoundMode::Test).unwrap();
        assert_eq!(round, 2183662);
    }

    #[test]
    fn next_round_works_for_time_mode() {
        let mut deps = mock_dependencies();

        // UNIX epoch
        let round = next_round(
            &mut deps.storage,
            NextRoundMode::Time {
                base: Timestamp::from_seconds(0),
            },
        )
        .unwrap();
        assert_eq!(round, 1);

        // Before Drand genesis (https://api3.drand.sh/info)
        let round = next_round(
            &mut deps.storage,
            NextRoundMode::Time {
                base: Timestamp::from_seconds(1595431050).minus_nanos(1),
            },
        )
        .unwrap();
        assert_eq!(round, 1);

        // At Drand genesis (https://api3.drand.sh/info)
        let round = next_round(
            &mut deps.storage,
            NextRoundMode::Time {
                base: Timestamp::from_seconds(1595431050),
            },
        )
        .unwrap();
        assert_eq!(round, 2);

        // After Drand genesis (https://api3.drand.sh/info)
        let round = next_round(
            &mut deps.storage,
            NextRoundMode::Time {
                base: Timestamp::from_seconds(1595431050).plus_nanos(1),
            },
        )
        .unwrap();
        assert_eq!(round, 2);

        // Drand genesis +29s/30s/31s
        let round = next_round(
            &mut deps.storage,
            NextRoundMode::Time {
                base: Timestamp::from_seconds(1595431050).plus_seconds(29),
            },
        )
        .unwrap();
        assert_eq!(round, 2);
        let round = next_round(
            &mut deps.storage,
            NextRoundMode::Time {
                base: Timestamp::from_seconds(1595431050).plus_seconds(30),
            },
        )
        .unwrap();
        assert_eq!(round, 3);
        let round = next_round(
            &mut deps.storage,
            NextRoundMode::Time {
                base: Timestamp::from_seconds(1595431050).plus_seconds(31),
            },
        )
        .unwrap();
        assert_eq!(round, 3);
    }
}
