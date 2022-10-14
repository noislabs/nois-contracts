#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Attribute, Deps, DepsMut, Env, Event, Ibc3ChannelOpenResponse,
    IbcBasicResponse, IbcChannelCloseMsg, IbcChannelConnectMsg, IbcChannelOpenMsg, IbcMsg,
    IbcPacketAckMsg, IbcPacketReceiveMsg, IbcPacketTimeoutMsg, IbcReceiveResponse, MessageInfo,
    QueryResponse, Reply, Response, StdError, StdResult, Storage, SubMsg, SubMsgResult, Timestamp,
    WasmMsg,
};
use nois::{NoisCallback, ReceiverExecuteMsg};
use nois_protocol::{
    check_order, check_version, DeliverBeaconPacket, DeliverBeaconPacketAck, RequestBeaconPacket,
    RequestBeaconPacketAck, StdAck,
};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Config, CONFIG, ORACLE_CHANNEL};

// TODO: make configurable?
/// packets live one hour
pub const PACKET_LIFETIME: u64 = 60 * 60;
pub const CALLBACK_ID: u64 = 456;

pub const SAFETY_MARGIN: u64 = 3; // seconds

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response> {
    let config = Config {};
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::GetNextRandomness { job_id } => {
            execute_get_next_randomness(deps, env, info, job_id)
        }
        ExecuteMsg::GetRandomnessAfter { after, job_id } => {
            execute_get_randomness_after(deps, env, info, after, job_id)
        }
    }
}

pub fn execute_get_next_randomness(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    job_id: String,
) -> Result<Response, ContractError> {
    let sender = info.sender.into();

    let packet = RequestBeaconPacket {
        after: env.block.time.plus_seconds(SAFETY_MARGIN),
        sender,
        job_id,
    };
    let channel_id = get_oracle_channel(deps.storage)?;
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

pub fn execute_get_randomness_after(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    after: Timestamp,
    job_id: String,
) -> Result<Response, ContractError> {
    let sender = info.sender.into();

    let packet = RequestBeaconPacket {
        after,
        sender,
        job_id,
    };
    let channel_id = get_oracle_channel(deps.storage)?;
    let msg = IbcMsg::SendPacket {
        channel_id,
        data: to_binary(&packet)?,
        timeout: env.block.time.plus_seconds(PACKET_LIFETIME).into(),
    };

    let res = Response::new()
        .add_message(msg)
        .add_attribute("action", "execute_get_randomness_after");
    Ok(res)
}

fn get_oracle_channel(storage: &dyn Storage) -> Result<String, ContractError> {
    let data = ORACLE_CHANNEL.may_load(storage)?;
    match data {
        Some(d) => Ok(d),
        None => Err(ContractError::UnsetChannel),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, reply: Reply) -> StdResult<Response> {
    match reply.id {
        CALLBACK_ID => {
            let mut attributes = vec![];
            match reply.result {
                SubMsgResult::Ok(_) => attributes.push(Attribute::new("success", "true")),
                SubMsgResult::Err(err) => {
                    attributes.push(Attribute::new("success", "false"));
                    attributes.push(Attribute::new("log", err));
                }
            };
            let callback_event = Event::new("nois-callback").add_attributes(attributes);
            Ok(Response::new().add_event(callback_event))
        }
        _ => Err(StdError::generic_err("invalid reply id or result")),
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

    ORACLE_CHANNEL.save(deps.storage, channel_id)?;

    Ok(IbcBasicResponse::new()
        .add_attribute("action", "ibc_connect")
        .add_attribute("channel_id", channel_id))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_channel_close(
    deps: DepsMut,
    _env: Env,
    msg: IbcChannelCloseMsg,
) -> StdResult<IbcBasicResponse> {
    let channel = msg.channel();

    ORACLE_CHANNEL.remove(deps.storage);

    // remove the channel
    let channel_id = &channel.endpoint.channel_id;

    Ok(IbcBasicResponse::new()
        .add_attribute("action", "ibc_close")
        .add_attribute("channel_id", channel_id))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_packet_receive(
    _deps: DepsMut,
    _env: Env,
    packet: IbcPacketReceiveMsg,
) -> StdResult<IbcReceiveResponse> {
    let DeliverBeaconPacket {
        source_id: _,
        randomness,
        sender,
        job_id,
    } = from_binary(&packet.packet.data)?;

    // Create the message for executing the callback.
    // This can fail for various reasons, like
    // - `sender` not being a contract
    // - the contract does not provide the Receive {} interface
    // - out of gas
    // - any other processing error in the callback implementation
    let msg = SubMsg::reply_on_error(
        WasmMsg::Execute {
            contract_addr: sender,
            msg: to_binary(&ReceiverExecuteMsg::Receive {
                callback: NoisCallback {
                    job_id: job_id.clone(),
                    randomness,
                },
            })?,
            funds: vec![],
        },
        CALLBACK_ID,
    )
    .with_gas_limit(2_000_000);

    let acknowledgement = StdAck::success(&DeliverBeaconPacketAck::Delivered {
        job_id: job_id.clone(),
    });
    Ok(IbcReceiveResponse::new()
        .set_ack(acknowledgement)
        .add_attribute("action", "acknowledge_ibc_query")
        .add_attribute("job_id", job_id)
        .add_submessage(msg))
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
    use nois_protocol::{APP_ORDER, BAD_APP_ORDER, IBC_APP_VERSION};

    const CREATOR: &str = "creator";

    fn setup() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {};
        let info = mock_info(CREATOR, &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        deps
    }

    #[test]
    fn instantiate_works() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {};
        let info = mock_info(CREATOR, &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn ibc_channel_open_checks_version_and_order() {
        let mut deps = setup();

        // All good
        let valid_handshake = mock_ibc_channel_open_try("channel-12", APP_ORDER, IBC_APP_VERSION);
        ibc_channel_open(deps.as_mut(), mock_env(), valid_handshake).unwrap();

        // Wrong order
        let wrong_order = mock_ibc_channel_open_try("channel-12", BAD_APP_ORDER, IBC_APP_VERSION);
        let res = ibc_channel_open(deps.as_mut(), mock_env(), wrong_order).unwrap_err();
        assert!(matches!(res, ContractError::ChannelError(..)));

        // Wrong version
        let wrong_version = mock_ibc_channel_open_try("channel-12", APP_ORDER, "another version");
        let res = ibc_channel_open(deps.as_mut(), mock_env(), wrong_version).unwrap_err();
        assert!(matches!(res, ContractError::ChannelError(..)));
    }
}
