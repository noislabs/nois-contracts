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
use crate::job_id::validate_job_id;
use crate::msg::{ExecuteMsg, InstantiateMsg, OracleChannelResponse, QueryMsg};
use crate::publish_time::{calculate_after, AfterMode};
use crate::state::{Config, CONFIG, ORACLE_CHANNEL};

// TODO: make configurable?
/// packets live one hour
pub const PACKET_LIFETIME: u64 = 60 * 60;
pub const CALLBACK_ID: u64 = 456;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let InstantiateMsg { test_mode } = msg;
    let config = Config { test_mode };
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("test_mode", test_mode.to_string()))
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
    validate_job_id(&job_id)?;

    let config = CONFIG.load(deps.storage)?;
    let mode = if config.test_mode {
        AfterMode::Test
    } else {
        AfterMode::BlockTime(env.block.time)
    };
    let after = calculate_after(deps.storage, mode)?;

    let packet = RequestBeaconPacket {
        after,
        sender: info.sender.into(),
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
    validate_job_id(&job_id)?;

    let packet = RequestBeaconPacket {
        after,
        sender: info.sender.into(),
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
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<QueryResponse> {
    match msg {
        QueryMsg::OracleChannel {} => to_binary(&query_oracle_channel(deps)?),
    }
}

fn query_oracle_channel(deps: Deps) -> StdResult<OracleChannelResponse> {
    Ok(OracleChannelResponse {
        channel: ORACLE_CHANNEL.may_load(deps.storage)?,
    })
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
) -> Result<IbcBasicResponse, ContractError> {
    let channel = msg.channel();
    let channel_id = &channel.endpoint.channel_id;

    if ORACLE_CHANNEL.may_load(deps.storage)?.is_some() {
        return Err(ContractError::ChannelAlreadySet);
    }

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
) -> Result<IbcBasicResponse, ContractError> {
    match msg {
        // This side of the channel never initiates a close.
        // Transactions trying that should fail.
        IbcChannelCloseMsg::CloseInit { channel: _ } => Err(ContractError::ChannelMustNotBeClosed),
        // If the close is already done on the other chain we cannot
        // stop that anymore. We ensure this transactions succeeds to
        // allow the local channel's state to change to closed.
        //
        // By clearing the ORACLE_CHANNEL we allow a new channel to be established.
        IbcChannelCloseMsg::CloseConfirm { channel } => {
            ORACLE_CHANNEL.remove(deps.storage);
            Ok(IbcBasicResponse::new()
                .add_attribute("action", "ibc_close")
                .add_attribute("channel_id", channel.endpoint.channel_id))
        }
    }
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
            mock_dependencies, mock_env, mock_ibc_channel_close_confirm,
            mock_ibc_channel_close_init, mock_ibc_channel_connect_ack,
            mock_ibc_channel_connect_confirm, mock_ibc_channel_open_try, mock_info, MockApi,
            MockQuerier, MockStorage,
        },
        CosmosMsg, OwnedDeps, ReplyOn,
    };
    use nois_protocol::{APP_ORDER, BAD_APP_ORDER, IBC_APP_VERSION};

    const CREATOR: &str = "creator";

    fn setup() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg { test_mode: true };
        let info = mock_info(CREATOR, &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        deps
    }

    fn setup_channel(mut deps: DepsMut) {
        let open_try = mock_ibc_channel_open_try("channel-12", APP_ORDER, IBC_APP_VERSION);
        ibc_channel_open(deps.branch(), mock_env(), open_try).unwrap();

        let connect_ack = mock_ibc_channel_connect_ack("channel-12", APP_ORDER, IBC_APP_VERSION);
        ibc_channel_connect(deps, mock_env(), connect_ack).unwrap();
    }

    #[test]
    fn instantiate_works() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg { test_mode: false };
        let info = mock_info(CREATOR, &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    //
    // Execute tests
    //

    #[test]
    fn get_next_randomness_works() {
        let mut deps = setup();

        // Requires a channel to forward requests to
        setup_channel(deps.as_mut());

        let msg = ExecuteMsg::GetNextRandomness {
            job_id: "foo".to_string(),
        };
        let res = execute(deps.as_mut(), mock_env(), mock_info("dapp", &[]), msg).unwrap();
        assert_eq!(res.messages.len(), 1);
        let out_msg = &res.messages[0];
        assert_eq!(out_msg.gas_limit, None);
        assert_eq!(out_msg.reply_on, ReplyOn::Never);
        assert!(matches!(
            out_msg.msg,
            CosmosMsg::Ibc(IbcMsg::SendPacket { .. })
        ));
    }

    #[test]
    fn get_next_randomnes_for_invalid_inputs() {
        let mut deps = setup();
        setup_channel(deps.as_mut());

        // Job ID too long
        let msg = ExecuteMsg::GetNextRandomness {
            job_id: "cb480eb3697f39db828d9efa021abe681bfcd72e23894019b8ddb1ab94039081-and-counting"
                .to_string(),
        };
        let err = execute(deps.as_mut(), mock_env(), mock_info("dapp", &[]), msg).unwrap_err();
        assert!(matches!(err, ContractError::JobIdTooLong));
    }

    #[test]
    fn get_randomness_after_works() {
        let mut deps = setup();

        // Requires a channel to forward requests to
        setup_channel(deps.as_mut());

        let msg = ExecuteMsg::GetRandomnessAfter {
            after: Timestamp::from_seconds(1666343642),
            job_id: "foo".to_string(),
        };
        let res = execute(deps.as_mut(), mock_env(), mock_info("dapp", &[]), msg).unwrap();
        assert_eq!(res.messages.len(), 1);
        let out_msg = &res.messages[0];
        assert_eq!(out_msg.gas_limit, None);
        assert_eq!(out_msg.reply_on, ReplyOn::Never);
        assert!(matches!(
            out_msg.msg,
            CosmosMsg::Ibc(IbcMsg::SendPacket { .. })
        ));
    }

    #[test]
    fn get_randomness_after_fails_for_invalid_inputs() {
        let mut deps = setup();
        setup_channel(deps.as_mut());

        // Job ID too long
        let msg = ExecuteMsg::GetRandomnessAfter {
            after: Timestamp::from_seconds(1666343642),
            job_id: "cb480eb3697f39db828d9efa021abe681bfcd72e23894019b8ddb1ab94039081-and-counting"
                .to_string(),
        };
        let err = execute(deps.as_mut(), mock_env(), mock_info("dapp", &[]), msg).unwrap_err();
        assert!(matches!(err, ContractError::JobIdTooLong));
    }

    //
    // IBC tests
    //

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

    #[test]
    fn ibc_channel_connect_works() {
        // We are chain A and get the ChanOpenAck
        {
            let mut deps = setup();

            // Channel is unset
            let OracleChannelResponse { channel } =
                from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::OracleChannel {}).unwrap())
                    .unwrap();
            assert_eq!(channel, None);

            let msg = mock_ibc_channel_connect_ack("channel-12", APP_ORDER, IBC_APP_VERSION);
            ibc_channel_connect(deps.as_mut(), mock_env(), msg).unwrap();

            // Channel is now set
            let OracleChannelResponse { channel } =
                from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::OracleChannel {}).unwrap())
                    .unwrap();
            assert_eq!(channel, Some("channel-12".to_string()));

            // One more ChanOpenAck
            let msg = mock_ibc_channel_connect_ack("channel-12", APP_ORDER, IBC_APP_VERSION);
            let err = ibc_channel_connect(deps.as_mut(), mock_env(), msg).unwrap_err();
            assert!(matches!(err, ContractError::ChannelAlreadySet));

            // Or an ChanOpenConfirm
            let msg = mock_ibc_channel_connect_confirm("channel-12", APP_ORDER, IBC_APP_VERSION);
            let err = ibc_channel_connect(deps.as_mut(), mock_env(), msg).unwrap_err();
            assert!(matches!(err, ContractError::ChannelAlreadySet));
        }

        // We are chain B and get the ChanOpenConfirm
        {
            let mut deps = setup();

            // Channel is unset
            let OracleChannelResponse { channel } =
                from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::OracleChannel {}).unwrap())
                    .unwrap();
            assert_eq!(channel, None);

            let msg = mock_ibc_channel_connect_confirm("channel-12", APP_ORDER, IBC_APP_VERSION);
            ibc_channel_connect(deps.as_mut(), mock_env(), msg).unwrap();

            // Channel is now set
            let OracleChannelResponse { channel } =
                from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::OracleChannel {}).unwrap())
                    .unwrap();
            assert_eq!(channel, Some("channel-12".to_string()));

            // One more ChanOpenConfirm
            let msg = mock_ibc_channel_connect_confirm("channel-12", APP_ORDER, IBC_APP_VERSION);
            let err = ibc_channel_connect(deps.as_mut(), mock_env(), msg).unwrap_err();
            assert!(matches!(err, ContractError::ChannelAlreadySet));

            // Or an ChanOpenAck
            let msg = mock_ibc_channel_connect_ack("channel-12", APP_ORDER, IBC_APP_VERSION);
            let err = ibc_channel_connect(deps.as_mut(), mock_env(), msg).unwrap_err();
            assert!(matches!(err, ContractError::ChannelAlreadySet));
        }
    }

    #[test]
    fn ibc_channel_close_works() {
        let mut deps = setup();

        // Open
        let valid_handshake = mock_ibc_channel_open_try("channel-12", APP_ORDER, IBC_APP_VERSION);
        ibc_channel_open(deps.as_mut(), mock_env(), valid_handshake).unwrap();

        // Connect
        let msg = mock_ibc_channel_connect_confirm("channel-12", APP_ORDER, IBC_APP_VERSION);
        ibc_channel_connect(deps.as_mut(), mock_env(), msg).unwrap();

        // Channel is now set
        let OracleChannelResponse { channel } =
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::OracleChannel {}).unwrap())
                .unwrap();
        assert_eq!(channel, Some("channel-12".to_string()));

        // Closing channel fails
        let msg = mock_ibc_channel_close_init("channel-12", APP_ORDER, IBC_APP_VERSION);
        let err = ibc_channel_close(deps.as_mut(), mock_env(), msg).unwrap_err();
        assert!(matches!(err, ContractError::ChannelMustNotBeClosed));

        // Channel is still set
        let OracleChannelResponse { channel } =
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::OracleChannel {}).unwrap())
                .unwrap();
        assert_eq!(channel, Some("channel-12".to_string()));

        // The other side closed
        let msg = mock_ibc_channel_close_confirm("channel-12", APP_ORDER, IBC_APP_VERSION);
        ibc_channel_close(deps.as_mut(), mock_env(), msg).unwrap();

        // Channel is unset
        let OracleChannelResponse { channel } =
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::OracleChannel {}).unwrap())
                .unwrap();
        assert_eq!(channel, None);
    }
}
