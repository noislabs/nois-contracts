use anything::Anything;
use cosmwasm_std::{
    attr, ensure_eq, from_binary, from_slice, to_binary, Addr, Attribute, BankMsg, Binary, Coin,
    CosmosMsg, Deps, DepsMut, Env, Event, HexBinary, Ibc3ChannelOpenResponse, IbcBasicResponse,
    IbcChannelCloseMsg, IbcChannelConnectMsg, IbcChannelOpenMsg, IbcMsg, IbcPacketAckMsg,
    IbcPacketReceiveMsg, IbcPacketTimeoutMsg, IbcReceiveResponse, MessageInfo, Never,
    QueryResponse, Reply, Response, StdError, StdResult, Storage, SubMsg, SubMsgResult, Timestamp,
    Uint128, WasmMsg,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{entry_point, Empty};
use nois::{NoisCallback, ReceiverExecuteMsg};
use nois_protocol::{
    check_order, check_version, InPacket, InPacketAck, OutPacket, OutPacketAck, StdAck,
    REQUEST_BEACON_PACKET_LIFETIME, TRANSFER_PACKET_LIFETIME,
};

use crate::error::ContractError;
use crate::jobs::{validate_job_id, validate_payment};
use crate::msg::{
    ConfigResponse, ExecuteMsg, GatewayChannelResponse, InstantiateMsg, PriceResponse,
    PricesResponse, QueryMsg, RequestBeaconOrigin, SudoMsg,
};
use crate::publish_time::{calculate_after, AfterMode};
use crate::state::{Config, OperationalMode, CONFIG, GATEWAY_CHANNEL};

pub const CALLBACK_ID: u64 = 456;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let InstantiateMsg {
        prices,
        manager,
        test_mode,
        callback_gas_limit,
        mode,
    } = msg;
    let manager = match manager {
        Some(ma) => Some(deps.api.addr_validate(&ma)?),
        None => None,
    };
    let config = Config {
        prices,
        manager,
        test_mode,
        callback_gas_limit,
        payment: None,
        // We query the current price from IBC. As long as we don't have it, we pay nothing.
        nois_beacon_price: Uint128::zero(),
        nois_beacon_price_updated: Timestamp::from_seconds(0),
        mode,
    };
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("test_mode", test_mode.to_string()))
}

// This no-op migrate implementation allows us to upgrade within the 0.7 series.
// No state changes expected.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: Empty) -> StdResult<Response> {
    Ok(Response::default())
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
        ExecuteMsg::SetConfig {
            manager,
            prices,
            payment,
            nois_beacon_price,
            mode,
        } => execute_set_config(
            deps,
            info,
            env,
            manager,
            prices,
            payment,
            nois_beacon_price,
            mode,
        ),
        ExecuteMsg::GetRandomnessAfter { after, job_id } => {
            execute_get_randomness_after(deps, env, info, after, job_id)
        }
        ExecuteMsg::Withdaw {
            denom,
            amount,
            address,
        } => execute_withdraw(deps, env, info, denom, amount, address),
    }
}

fn execute_get_next_randomness(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    job_id: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let mode = if config.test_mode {
        AfterMode::Test
    } else {
        AfterMode::BlockTime(env.block.time)
    };
    let after = calculate_after(deps.storage, mode)?;

    execute_get_randomness_impl(
        deps,
        env,
        info,
        config,
        after,
        job_id,
        "execute_get_next_randomness",
    )
}

fn execute_get_randomness_after(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    after: Timestamp,
    job_id: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    execute_get_randomness_impl(
        deps,
        env,
        info,
        config,
        after,
        job_id,
        "execute_get_randomness_after",
    )
}

pub fn execute_get_randomness_impl(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    config: Config,
    after: Timestamp,
    job_id: String,
    action: &str,
) -> Result<Response, ContractError> {
    validate_job_id(&job_id)?;
    validate_payment(&config.prices, &info.funds)?;

    let packet = InPacket::RequestBeacon {
        after,
        origin: to_binary(&RequestBeaconOrigin {
            sender: info.sender.into(),
            job_id,
        })?,
    };
    let channel_id = get_gateway_channel(deps.storage)?;

    let mut msgs: Vec<CosmosMsg> = Vec::with_capacity(2);

    // Add payment frist such that (at least in integration tests) the funds arrive in time
    if let OperationalMode::IbcPay { unois_denom } = config.mode {
        if let Some(payment_contract) = config.payment {
            if !config.nois_beacon_price.is_zero() {
                msgs.push(
                    IbcMsg::Transfer {
                        channel_id: unois_denom.ics20_channel,
                        to_address: payment_contract,
                        amount: Coin {
                            amount: config.nois_beacon_price,
                            denom: unois_denom.denom,
                        },
                        timeout: env.block.time.plus_seconds(TRANSFER_PACKET_LIFETIME).into(),
                    }
                    .into(),
                );
            }
        }
    }

    msgs.push(
        IbcMsg::SendPacket {
            channel_id,
            data: to_binary(&packet)?,
            timeout: env
                .block
                .time
                .plus_seconds(REQUEST_BEACON_PACKET_LIFETIME)
                .into(),
        }
        .into(),
    );

    let res = Response::new()
        .add_messages(msgs)
        .add_attribute("action", action);
    Ok(res)
}

#[allow(clippy::too_many_arguments)]
fn execute_set_config(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    manager: Option<String>,
    prices: Option<Vec<Coin>>,
    payment: Option<String>,
    nois_beacon_price: Option<Uint128>,
    mode: Option<OperationalMode>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // if manager set, check the calling address is the authorised multisig otherwise error unauthorised
    ensure_eq!(
        &info.sender,                                                // &Addr
        config.manager.as_ref().ok_or(ContractError::Unauthorized)?, // &Addr
        ContractError::Unauthorized
    );

    set_config_unchecked(deps, env, manager, prices, payment, nois_beacon_price, mode)
}

fn execute_withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    denom: String,
    amount: Option<Uint128>,
    address: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // if manager set, check the calling address is the authorised multisig otherwise error unauthorised
    ensure_eq!(
        &info.sender,                                                // &Addr
        config.manager.as_ref().ok_or(ContractError::Unauthorized)?, // &Addr
        ContractError::Unauthorized
    );

    withdraw_unchecked(deps, env, denom, amount, address)
}

#[cfg_attr(not(feature = "library"), entry_point)]
#[allow(unused)]
pub fn sudo(deps: DepsMut, env: Env, msg: SudoMsg) -> Result<Response, ContractError> {
    match msg {
        #[cfg(feature = "governance_owned")]
        SudoMsg::Withdaw {
            denom,
            amount,
            address,
        } => withdraw_unchecked(deps, env, denom, amount, address),
        #[cfg(feature = "governance_owned")]
        SudoMsg::WithdrawToCommunityPool { denom, amount } => {
            withdraw_community_pool_unchecked(deps, env, denom, amount)
        }
        #[cfg(feature = "governance_owned")]
        SudoMsg::SetConfig {
            manager,
            prices,
            payment,
            nois_beacon_price,
            mode,
        } => set_config_unchecked(deps, env, manager, prices, payment, nois_beacon_price, mode),
    }
}

fn withdraw_unchecked(
    deps: DepsMut,
    env: Env,
    denom: String,
    amount: Option<Uint128>,
    address: String,
) -> Result<Response, ContractError> {
    let address = deps.api.addr_validate(&address)?;
    let amount: Coin = match amount {
        Some(amount) => Coin { denom, amount },
        None => deps.querier.query_balance(env.contract.address, denom)?,
    };

    let msg = BankMsg::Send {
        to_address: address.into(),
        amount: vec![amount.clone()],
    };
    let res = Response::new()
        .add_message(msg)
        .add_attribute("action", "withdraw")
        .add_attribute("amount", amount.to_string());
    Ok(res)
}

#[allow(unused)]
fn withdraw_community_pool_unchecked(
    deps: DepsMut,
    env: Env,
    denom: String,
    amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    let amount: Coin = match amount {
        Some(amount) => Coin { denom, amount },
        None => deps
            .querier
            .query_balance(env.contract.address.clone(), denom)?,
    };

    let msg = CosmosMsg::Stargate {
        type_url: "/cosmos.distribution.v1beta1.MsgFundCommunityPool".to_string(),
        value: encode_msg_fund_community_pool(&amount, &env.contract.address).into(),
    };

    let res = Response::new()
        .add_message(msg)
        .add_attribute("action", "withdraw_community_pool")
        .add_attribute("amount", amount.to_string());
    Ok(res)
}

fn set_config_unchecked(
    deps: DepsMut,
    env: Env,
    manager: Option<String>,
    prices: Option<Vec<Coin>>,
    payment: Option<String>,
    nois_beacon_price: Option<Uint128>,
    mode: Option<OperationalMode>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let manager = match manager {
        Some(ma) => Some(deps.api.addr_validate(&ma)?),
        None => config.manager,
    };
    let prices = prices.unwrap_or(config.prices);

    let test_mode = config.test_mode;
    let callback_gas_limit = config.callback_gas_limit;
    let payment = match payment {
        Some(pa) => Some(pa),
        None => config.payment,
    };
    let (nois_beacon_price, nois_beacon_price_updated) = match nois_beacon_price {
        Some(bp) => (bp, env.block.time),
        None => (config.nois_beacon_price, config.nois_beacon_price_updated),
    };
    let mode = mode.unwrap_or(config.mode);

    let new_config = Config {
        manager,
        prices,
        test_mode,
        callback_gas_limit,
        payment,
        nois_beacon_price,
        nois_beacon_price_updated,
        mode,
    };

    CONFIG.save(deps.storage, &new_config)?;

    Ok(Response::default())
}

fn get_gateway_channel(storage: &dyn Storage) -> Result<String, ContractError> {
    let data = GATEWAY_CHANNEL.may_load(storage)?;
    match data {
        Some(d) => Ok(d),
        None => Err(ContractError::UnsetChannel),
    }
}

#[allow(unused)]
fn encode_msg_fund_community_pool(amount: &Coin, depositor: &Addr) -> Vec<u8> {
    // Coin: https://github.com/cosmos/cosmos-sdk/blob/v0.45.15/proto/cosmos/base/v1beta1/coin.proto#L14-L19
    // MsgFundCommunityPool: https://github.com/cosmos/cosmos-sdk/blob/v0.45.15/proto/cosmos/distribution/v1beta1/tx.proto#L69-L76
    let coin = Anything::new()
        .append_bytes(1, &amount.denom)
        .append_bytes(2, amount.amount.to_string());
    Anything::new()
        .append_message(1, &coin)
        .append_bytes(2, depositor.as_bytes())
        .into_vec()
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
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::Prices {} => to_binary(&query_prices(deps)?),
        QueryMsg::Price { denom } => to_binary(&query_price(deps, denom)?),
        QueryMsg::GatewayChannel {} => to_binary(&query_gateway_channel(deps)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse { config })
}

fn query_prices(deps: Deps) -> StdResult<PricesResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(PricesResponse {
        prices: config.prices,
    })
}

fn query_price(deps: Deps, denom: String) -> StdResult<PriceResponse> {
    let config = CONFIG.load(deps.storage)?;
    let price = config
        .prices
        .into_iter()
        .find(|price| price.denom == denom)
        .map(|coin| coin.amount);
    Ok(PriceResponse { price })
}

fn query_gateway_channel(deps: Deps) -> StdResult<GatewayChannelResponse> {
    Ok(GatewayChannelResponse {
        channel: GATEWAY_CHANNEL.may_load(deps.storage)?,
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
/// enforces ordering and versioing constraints
pub fn ibc_channel_open(
    _deps: DepsMut,
    _env: Env,
    msg: IbcChannelOpenMsg,
) -> Result<Option<Ibc3ChannelOpenResponse>, ContractError> {
    let channel = match msg {
        IbcChannelOpenMsg::OpenInit { channel } => channel,
        IbcChannelOpenMsg::OpenTry { .. } => return Err(ContractError::MustBeChainA),
    };

    check_order(&channel.order)?;
    check_version(&channel.version)?;

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
    let channel = match msg {
        IbcChannelConnectMsg::OpenAck {
            channel,
            counterparty_version: _,
        } => channel,
        IbcChannelConnectMsg::OpenConfirm { .. } => return Err(ContractError::MustBeChainA),
    };

    let channel_id = channel.endpoint.channel_id;

    if GATEWAY_CHANNEL.may_load(deps.storage)?.is_some() {
        return Err(ContractError::ChannelAlreadySet);
    }

    GATEWAY_CHANNEL.save(deps.storage, &channel_id)?;
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
        // By clearing the GATEWAY_CHANNEL we allow a new channel to be established.
        IbcChannelCloseMsg::CloseConfirm { channel } => {
            GATEWAY_CHANNEL.remove(deps.storage);
            Ok(IbcBasicResponse::new()
                .add_attribute("action", "ibc_close")
                .add_attribute("channel_id", channel.endpoint.channel_id))
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_packet_receive(
    deps: DepsMut,
    env: Env,
    msg: IbcPacketReceiveMsg,
) -> Result<IbcReceiveResponse, Never> {
    // put this in a closure so we can convert all error responses into acknowledgements
    (|| {
        let IbcPacketReceiveMsg { packet, .. } = msg;
        let op: OutPacket = from_binary(&packet.data)?;
        match op {
            OutPacket::DeliverBeacon {
                source_id: _,
                published,
                randomness,
                origin,
            } => receive_deliver_beacon(deps, published, randomness, origin),
            OutPacket::Welcome { payment } => receive_welcome(deps, env, payment),
            OutPacket::PushBeaconPrice {
                timestamp,
                amount,
                denom,
            } => receive_push_beacon_price(deps, env, timestamp, amount, denom),
            _ => Err(ContractError::UnsupportedPacketType),
        }
    })()
    .or_else(|e| {
        // we try to capture all app-level errors and convert them into
        // acknowledgement packets that contain an error code.
        let acknowledgement = StdAck::error(format!("Error processing packet: {e}"));
        Ok(IbcReceiveResponse::new()
            .set_ack(acknowledgement)
            .add_event(Event::new("ibc").add_attribute("packet", "receive")))
    })
}

fn receive_deliver_beacon(
    deps: DepsMut,
    published: Timestamp,
    randomness: HexBinary,
    origin: Binary,
) -> Result<IbcReceiveResponse, ContractError> {
    let Config {
        callback_gas_limit, ..
    } = CONFIG.load(deps.storage)?;

    let RequestBeaconOrigin { sender, job_id } = from_slice(&origin)?;

    // Create the message for executing the callback.
    // This can fail for various reasons, like
    // - `sender` not being a contract
    // - the contract does not provide the NoisReceive {} interface
    // - out of gas
    // - any other processing error in the callback implementation
    let msg = SubMsg::reply_on_error(
        WasmMsg::Execute {
            contract_addr: sender,
            msg: to_binary(&ReceiverExecuteMsg::NoisReceive {
                callback: NoisCallback {
                    job_id: job_id.clone(),
                    published,
                    randomness,
                },
            })?,
            funds: vec![],
        },
        CALLBACK_ID,
    )
    .with_gas_limit(callback_gas_limit);

    let ack = StdAck::success(OutPacketAck::DeliverBeacon {});
    Ok(IbcReceiveResponse::new()
        .set_ack(ack)
        .add_attribute("action", "acknowledge_ibc_query")
        .add_attribute("job_id", job_id)
        .add_submessage(msg))
}

fn receive_welcome(
    deps: DepsMut,
    _env: Env,
    payment: String,
) -> Result<IbcReceiveResponse, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    config.payment = Some(payment);
    CONFIG.save(deps.storage, &config)?;
    let ack = StdAck::success(OutPacketAck::Welcome {});
    Ok(IbcReceiveResponse::new().set_ack(ack))
}

fn receive_push_beacon_price(
    deps: DepsMut,
    _env: Env,
    timestamp: Timestamp,
    amount: Uint128,
    denom: String,
) -> Result<IbcReceiveResponse, ContractError> {
    update_nois_beacon_price(deps, timestamp, amount, denom)?;
    let ack = StdAck::success(OutPacketAck::PushBeaconPrice {});
    Ok(IbcReceiveResponse::new().set_ack(ack))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_packet_ack(
    deps: DepsMut,
    _env: Env,
    msg: IbcPacketAckMsg,
) -> Result<IbcBasicResponse, ContractError> {
    let mut attributes = Vec::<Attribute>::new();
    attributes.push(attr("action", "ack"));
    let ack: StdAck = from_binary(&msg.acknowledgement.data)?;
    let is_error: bool;
    match ack {
        StdAck::Result(data) => {
            is_error = false;
            let response: InPacketAck = from_binary(&data)?;
            let ack_type: String = match response {
                InPacketAck::RequestProcessed { source_id: _ } => "request_processed".to_string(),
                InPacketAck::RequestQueued { source_id: _ } => "request_queued".to_string(),
                InPacketAck::PullBeaconPrice {
                    timestamp,
                    amount,
                    denom,
                } => {
                    update_nois_beacon_price(deps, timestamp, amount, denom)?;
                    "beacon_price".to_string()
                }
                _ => "other".to_string(),
            };
            attributes.push(attr("ack_type", ack_type));
        }
        StdAck::Error(err) => {
            // The Request Beacon IBC packet failed, e.g. because the requested round
            // is too old. Here we should send the dapp an error callback as the randomness
            // will never come. Unfortunately we cannot map this packet to the job because
            // we don't know the sequence when emitting a IbcMsg::SendPacket.
            // https://github.com/CosmWasm/wasmd/issues/1154
            is_error = true;
            attributes.push(attr("error", err));
        }
    }
    attributes.push(attr("is_error", is_error.to_string()));
    Ok(IbcBasicResponse::new().add_attributes(attributes))
}

fn update_nois_beacon_price(
    deps: DepsMut,
    timestamp: Timestamp,
    new_price: Uint128,
    denom: String,
) -> Result<(), ContractError> {
    if denom != "unois" {
        // We don't understand the denom of this price. Ignore the price info.
        return Ok(());
    }

    let mut config = CONFIG.load(deps.storage)?;
    if config.nois_beacon_price_updated > timestamp {
        // We just got an older information than we already have
        return Ok(());
    }

    config.nois_beacon_price = new_price;
    config.nois_beacon_price_updated = timestamp;
    CONFIG.save(deps.storage, &config)?;
    Ok(())
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
    use crate::state::OperationalMode;

    use super::*;
    use cosmwasm_std::{
        coins,
        testing::{
            mock_dependencies, mock_dependencies_with_balance, mock_env,
            mock_ibc_channel_close_confirm, mock_ibc_channel_close_init,
            mock_ibc_channel_connect_ack, mock_ibc_channel_connect_confirm,
            mock_ibc_channel_open_init, mock_ibc_packet_ack, mock_info, MockApi, MockQuerier,
            MockStorage,
        },
        CosmosMsg, IbcAcknowledgement, OwnedDeps, ReplyOn, Uint128,
    };
    use nois_protocol::{InPacketAck, APP_ORDER, BAD_APP_ORDER, IBC_APP_VERSION};

    const CREATOR: &str = "creator";

    fn setup() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
        let initial_funds = vec![
            Coin::new(22334455, "unoisx"),
            Coin::new(
                123321,
                "ibc/CB480EB3697F39DB828D9EFA021ABE681BFCD72E23894019B8DDB1AB94039081",
            ),
        ];
        let mut deps = mock_dependencies_with_balance(&initial_funds);
        let msg = InstantiateMsg {
            manager: Some(CREATOR.to_string()),
            prices: vec![Coin::new(1_000000, "unoisx")],
            test_mode: true,
            callback_gas_limit: 500_000,
            mode: OperationalMode::Funded {},
        };
        let info = mock_info(CREATOR, &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        deps
    }

    /// Gets the value of the first attribute with the given key
    fn first_attr(data: impl AsRef<[Attribute]>, search_key: &str) -> Option<String> {
        data.as_ref().iter().find_map(|a| {
            if a.key == search_key {
                Some(a.value.clone())
            } else {
                None
            }
        })
    }

    fn setup_channel(mut deps: DepsMut) {
        let init = mock_ibc_channel_open_init("channel-12", APP_ORDER, IBC_APP_VERSION);
        ibc_channel_open(deps.branch(), mock_env(), init).unwrap();

        let ack = mock_ibc_channel_connect_ack("channel-12", APP_ORDER, IBC_APP_VERSION);
        ibc_channel_connect(deps, mock_env(), ack).unwrap();
    }

    #[test]
    fn instantiate_works() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            manager: Some(CREATOR.to_string()),
            prices: vec![Coin::new(1_000000, "unoisx")],
            test_mode: false,
            callback_gas_limit: 500_000,
            mode: OperationalMode::Funded {},
        };
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
        let info = mock_info("dapp", &coins(22334455, "unoisx"));
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
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
        let info = mock_info("dapp", &coins(22334455, "unoisx"));
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
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

    #[test]
    fn when_manager_is_not_set_manager_permissions_are_unathorised() {
        // Check that if manager not set, a random person cannot execute manager-like operations.
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            manager: None,
            prices: vec![Coin::new(1_000000, "unoisx")],
            test_mode: false,
            callback_gas_limit: 500_000,
            mode: OperationalMode::Funded {},
        };
        let info = mock_info(CREATOR, &[]);
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        // withdraw
        let msg = ExecuteMsg::Withdaw {
            denom: "unoisx".to_string(),
            amount: Some(Uint128::new(12)),
            address: "some-address".to_string(),
        };
        let err = execute(deps.as_mut(), mock_env(), mock_info("dapp", &[]), msg).unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized));
        // withdraw all
        let msg = ExecuteMsg::Withdaw {
            denom: "unoisx".to_string(),
            amount: None,
            address: "some-address".to_string(),
        };

        let err = execute(deps.as_mut(), mock_env(), mock_info("dapp", &[]), msg).unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized));
        // Edit config
        let msg = ExecuteMsg::SetConfig {
            manager: Some("some-manager".to_string()),
            prices: None,
            payment: None,
            nois_beacon_price: None,
            mode: None,
        };

        let err = execute(deps.as_mut(), mock_env(), mock_info("dapp", &[]), msg).unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized));
    }

    #[test]
    fn withdraw_works() {
        let mut deps = setup();

        let msg = ExecuteMsg::Withdaw {
            denom: "unoisx".to_string(),
            amount: Some(Uint128::new(12)),
            address: "some-address".to_string(),
        };
        let err = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("dapp", &[]),
            msg.clone(),
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized));
        let res = execute(deps.as_mut(), mock_env(), mock_info(CREATOR, &[]), msg).unwrap();
        assert_eq!(res.messages.len(), 1);
        assert_eq!(
            res.messages[0].msg,
            CosmosMsg::Bank(BankMsg::Send {
                to_address: "some-address".to_string(),
                amount: coins(12, "unoisx"),
            })
        );
    }

    #[test]
    fn withdraw_all_works() {
        let mut deps = setup();

        let msg = ExecuteMsg::Withdaw {
            denom: "unoisx".to_string(),
            amount: None,
            address: "some-address".to_string(),
        };

        let err = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("dapp", &[]),
            msg.clone(),
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized));
        let res = execute(deps.as_mut(), mock_env(), mock_info(CREATOR, &[]), msg).unwrap();

        assert_eq!(res.messages.len(), 1);
        assert_eq!(
            res.messages[0].msg,
            CosmosMsg::Bank(BankMsg::Send {
                to_address: "some-address".to_string(),
                amount: coins(22334455, "unoisx"),
            })
        );
    }

    //
    // Query tests
    //

    #[test]
    fn query_prices_works() {
        let deps = setup();

        let PricesResponse { prices } =
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Prices {}).unwrap()).unwrap();
        assert_eq!(prices, coins(1000000, "unoisx"));
    }

    #[test]
    fn query_price_works() {
        let deps = setup();

        let PriceResponse { price } = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Price {
                    denom: "shitcoin".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(price, None);

        let PriceResponse { price } = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Price {
                    denom: "unoisx".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(price, Some(Uint128::new(1000000)));
    }

    //
    // IBC tests
    //

    #[test]
    fn ibc_channel_open_checks_version_and_order() {
        let mut deps = setup();

        // All good
        let valid_handshake = mock_ibc_channel_open_init("channel-12", APP_ORDER, IBC_APP_VERSION);
        ibc_channel_open(deps.as_mut(), mock_env(), valid_handshake).unwrap();

        // Wrong order
        let wrong_order = mock_ibc_channel_open_init("channel-12", BAD_APP_ORDER, IBC_APP_VERSION);
        let res = ibc_channel_open(deps.as_mut(), mock_env(), wrong_order).unwrap_err();
        assert!(matches!(res, ContractError::ChannelError(..)));

        // Wrong version
        let wrong_version = mock_ibc_channel_open_init("channel-12", APP_ORDER, "another version");
        let res = ibc_channel_open(deps.as_mut(), mock_env(), wrong_version).unwrap_err();
        assert!(matches!(res, ContractError::ChannelError(..)));
    }

    #[test]
    fn ibc_channel_connect_works() {
        // We are chain A and get the ChanOpenAck

        let mut deps = setup();

        // Channel is unset
        let GatewayChannelResponse { channel } =
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::GatewayChannel {}).unwrap())
                .unwrap();
        assert_eq!(channel, None);

        let msg = mock_ibc_channel_connect_ack("channel-12", APP_ORDER, IBC_APP_VERSION);
        ibc_channel_connect(deps.as_mut(), mock_env(), msg).unwrap();

        // Channel is now set
        let GatewayChannelResponse { channel } =
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::GatewayChannel {}).unwrap())
                .unwrap();
        assert_eq!(channel, Some("channel-12".to_string()));

        // One more ChanOpenAck
        let msg = mock_ibc_channel_connect_ack("channel-12", APP_ORDER, IBC_APP_VERSION);
        let err = ibc_channel_connect(deps.as_mut(), mock_env(), msg).unwrap_err();
        assert!(matches!(err, ContractError::ChannelAlreadySet));

        // ChanOpenConfirm is rejected
        let msg = mock_ibc_channel_connect_confirm("channel-12", APP_ORDER, IBC_APP_VERSION);
        let err = ibc_channel_connect(deps.as_mut(), mock_env(), msg).unwrap_err();
        assert!(matches!(err, ContractError::MustBeChainA));
    }

    #[test]
    fn ibc_channel_close_works() {
        let mut deps = setup();

        // Open
        let valid_handshake = mock_ibc_channel_open_init("channel-12", APP_ORDER, IBC_APP_VERSION);
        ibc_channel_open(deps.as_mut(), mock_env(), valid_handshake).unwrap();

        // Connect
        let msg = mock_ibc_channel_connect_ack("channel-12", APP_ORDER, IBC_APP_VERSION);
        ibc_channel_connect(deps.as_mut(), mock_env(), msg).unwrap();

        // Channel is now set
        let GatewayChannelResponse { channel } =
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::GatewayChannel {}).unwrap())
                .unwrap();
        assert_eq!(channel, Some("channel-12".to_string()));

        // Closing channel fails
        let msg = mock_ibc_channel_close_init("channel-12", APP_ORDER, IBC_APP_VERSION);
        let err = ibc_channel_close(deps.as_mut(), mock_env(), msg).unwrap_err();
        assert!(matches!(err, ContractError::ChannelMustNotBeClosed));

        // Channel is still set
        let GatewayChannelResponse { channel } =
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::GatewayChannel {}).unwrap())
                .unwrap();
        assert_eq!(channel, Some("channel-12".to_string()));

        // The other side closed
        let msg = mock_ibc_channel_close_confirm("channel-12", APP_ORDER, IBC_APP_VERSION);
        ibc_channel_close(deps.as_mut(), mock_env(), msg).unwrap();

        // Channel is unset
        let GatewayChannelResponse { channel } =
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::GatewayChannel {}).unwrap())
                .unwrap();
        assert_eq!(channel, None);
    }

    #[test]
    fn ibc_packet_ack_works() {
        let mut deps = setup();

        // The proxy -> gateway packet we get the acknowledgement for
        let packet = InPacket::RequestBeacon {
            after: Timestamp::from_seconds(321),
            origin: to_binary(&RequestBeaconOrigin {
                sender: "contract345".to_string(),
                job_id: "hello".to_string(),
            })
            .unwrap(),
        };

        // Success ack (processed)
        let ack = StdAck::success(InPacketAck::RequestProcessed {
            source_id: "backend:123:456".to_string(),
        });
        let msg = mock_ibc_packet_ack(
            "channel-12",
            &packet,
            IbcAcknowledgement::encode_json(&ack).unwrap(),
        )
        .unwrap();
        let IbcBasicResponse { attributes, .. } =
            ibc_packet_ack(deps.as_mut(), mock_env(), msg).unwrap();
        assert_eq!(first_attr(&attributes, "action").unwrap(), "ack");
        assert_eq!(first_attr(&attributes, "is_error").unwrap(), "false");
        assert_eq!(first_attr(&attributes, "error"), None);
        assert_eq!(
            first_attr(&attributes, "ack_type").unwrap(),
            "request_processed"
        );

        // Success ack (queued)
        let ack = StdAck::success(InPacketAck::RequestQueued {
            source_id: "backend:123:456".to_string(),
        });
        let msg = mock_ibc_packet_ack(
            "channel-12",
            &packet,
            IbcAcknowledgement::encode_json(&ack).unwrap(),
        )
        .unwrap();
        let IbcBasicResponse { attributes, .. } =
            ibc_packet_ack(deps.as_mut(), mock_env(), msg).unwrap();
        assert_eq!(first_attr(&attributes, "action").unwrap(), "ack");
        assert_eq!(first_attr(&attributes, "is_error").unwrap(), "false");
        assert_eq!(first_attr(&attributes, "error"), None);
        assert_eq!(
            first_attr(&attributes, "ack_type").unwrap(),
            "request_queued"
        );

        // Error ack
        let ack = StdAck::error("kaputt");
        let msg = mock_ibc_packet_ack(
            "channel-12",
            &packet,
            IbcAcknowledgement::encode_json(&ack).unwrap(),
        )
        .unwrap();
        let IbcBasicResponse { attributes, .. } =
            ibc_packet_ack(deps.as_mut(), mock_env(), msg).unwrap();
        assert_eq!(first_attr(&attributes, "action").unwrap(), "ack");
        assert_eq!(first_attr(&attributes, "is_error").unwrap(), "true");
        assert_eq!(first_attr(&attributes, "error").unwrap(), "kaputt");
        assert_eq!(first_attr(&attributes, "ack_type"), None);
    }
}
