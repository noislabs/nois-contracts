use anybuf::Anybuf;
use cosmwasm_std::{
    attr, ensure_eq, from_binary, from_slice, to_binary, Addr, Attribute, BankMsg, Binary, Coin,
    CosmosMsg, Deps, DepsMut, Empty, Env, Event, HexBinary, Ibc3ChannelOpenResponse,
    IbcBasicResponse, IbcChannelCloseMsg, IbcChannelConnectMsg, IbcChannelOpenMsg, IbcMsg,
    IbcPacketAckMsg, IbcPacketReceiveMsg, IbcPacketTimeoutMsg, IbcReceiveResponse, MessageInfo,
    Never, Order, QueryResponse, Reply, Response, StdAck, StdResult, Storage, SubMsg, SubMsgResult,
    Timestamp, Uint128, WasmMsg,
};
use nois::{NoisCallback, ReceiverExecuteMsg};
use nois_protocol::{
    check_order, check_version, InPacket, InPacketAck, OutPacket, OutPacketAck,
    REQUEST_BEACON_PACKET_LIFETIME, TRANSFER_PACKET_LIFETIME,
};

use crate::attributes::{
    ATTR_ACTION, ATTR_CALLBACK_ERROR_MSG, ATTR_CALLBACK_SUCCESS, EVENT_TYPE_CALLBACK,
};
use crate::error::ContractError;
use crate::jobs::{validate_job_id, validate_payment};
use crate::msg::{
    AllowlistResponse, ConfigResponse, ExecuteMsg, GatewayChannelResponse, InstantiateMsg,
    IsAllowlistedResponse, PriceResponse, PricesResponse, QueryMsg, RequestBeaconOrigin, SudoMsg,
};
use crate::publish_time::{calculate_after, AfterMode};
use crate::state::{Config, OperationalMode, ALLOWLIST, ALLOWLIST_MARKER, CONFIG, GATEWAY_CHANNEL};

pub const REPLAY_ID_CALLBACK: u64 = 456;

/// 10 years in seconds
const TEN_YEARS_S: u64 = 10 * 3600 * 24 * 365;

/// If not set otherwise, min_after is the genesis time of Nois mainnet
const MIN_AFTER_FALLBACK: Timestamp = Timestamp::from_seconds(1680015600);
const MAX_AFTER_FALLBACK: Timestamp = MIN_AFTER_FALLBACK.plus_seconds(TEN_YEARS_S);

#[cfg_attr(not(feature = "library"), ::cosmwasm_std::entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let InstantiateMsg {
        prices,
        manager,
        test_mode,
        callback_gas_limit,
        mode,
        allowlist_enabled,
        allowlist,
    } = msg;
    let manager = match manager {
        Some(ma) => Some(deps.api.addr_validate(&ma)?),
        None => None,
    };
    let test_mode = test_mode.unwrap_or(false);
    let allowlist_enabled = allowlist_enabled.unwrap_or_default();
    let allowlist = allowlist.unwrap_or_default();

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
        allowlist_enabled: Some(allowlist_enabled),
        min_after: Some(env.block.time),
        max_after: Some(env.block.time.plus_seconds(TEN_YEARS_S)),
    };

    CONFIG.save(deps.storage, &config)?;

    // Save addresses to allow list.
    for addr in allowlist {
        let addr = deps.api.addr_validate(&addr)?;
        ALLOWLIST.save(deps.storage, &addr, &ALLOWLIST_MARKER)?;
    }

    Ok(Response::new()
        .add_attribute(ATTR_ACTION, "instantiate")
        .add_attribute("test_mode", test_mode.to_string()))
}

#[cfg_attr(not(feature = "library"), ::cosmwasm_std::entry_point)]
pub fn migrate(deps: DepsMut, env: Env, _msg: Empty) -> StdResult<Response> {
    let mut config = CONFIG.load(deps.storage)?;

    // If unset, set min_after and max_after to now and now+10y
    if config.min_after.is_none() {
        config.min_after = Some(env.block.time);
    }
    if config.max_after.is_none() {
        config.max_after = Some(env.block.time.plus_seconds(TEN_YEARS_S));
    }
    if config.allowlist_enabled.is_none() {
        config.allowlist_enabled = Some(false);
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default().add_attribute(ATTR_ACTION, "migrate"))
}

#[cfg_attr(not(feature = "library"), ::cosmwasm_std::entry_point)]
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
            callback_gas_limit,
            mode,
            allowlist_enabled,
            min_after,
            max_after,
        } => execute_set_config(
            deps,
            info,
            env,
            manager,
            prices,
            payment,
            nois_beacon_price,
            callback_gas_limit,
            mode,
            allowlist_enabled,
            min_after,
            max_after,
        ),
        ExecuteMsg::GetRandomnessAfter { after, job_id } => {
            execute_get_randomness_after(deps, env, info, after, job_id)
        }
        ExecuteMsg::Withdraw {
            denom,
            amount,
            address,
        } => execute_withdraw(deps, env, info, denom, amount, address),
        ExecuteMsg::UpdateAllowlist { add, remove } => {
            execute_update_allowlist(deps, env, info, add, remove)
        }
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
        "execute_get_next_randomness",
        config,
        after,
        job_id,
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
        "execute_get_randomness_after",
        config,
        after,
        job_id,
    )
}

pub fn execute_get_randomness_impl(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    action: &str,
    config: Config,
    after: Timestamp,
    job_id: String,
) -> Result<Response, ContractError> {
    validate_job_id(&job_id)?;
    validate_payment(&config.prices, &info.funds)?;

    // only let whitelisted senders get randomness
    let allowlist_enabled = config.allowlist_enabled.unwrap_or(false);
    if allowlist_enabled && !ALLOWLIST.has(deps.storage, &info.sender) {
        return Err(ContractError::SenderNotAllowed);
    }

    let min_after = config.min_after.unwrap_or(MIN_AFTER_FALLBACK);
    if after < min_after {
        return Err(ContractError::AfterTooLow { min_after, after });
    }

    let max_after = config.max_after.unwrap_or(MAX_AFTER_FALLBACK);
    if after > max_after {
        return Err(ContractError::AfterTooHigh { max_after, after });
    }

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
        .add_attribute(ATTR_ACTION, action);
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
    callback_gas_limit: Option<u64>,
    mode: Option<OperationalMode>,
    allowlist_enabled: Option<bool>,
    min_after: Option<Timestamp>,
    max_after: Option<Timestamp>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // if manager set, check the calling address is the authorised multisig otherwise error unauthorised
    ensure_eq!(
        info.sender,
        config.manager.as_ref().ok_or(ContractError::Unauthorized)?,
        ContractError::Unauthorized
    );

    set_config_unchecked(
        deps,
        env,
        "execute_set_config",
        manager,
        prices,
        payment,
        nois_beacon_price,
        callback_gas_limit,
        mode,
        allowlist_enabled,
        min_after,
        max_after,
    )
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
        info.sender,
        config.manager.as_ref().ok_or(ContractError::Unauthorized)?,
        ContractError::Unauthorized
    );

    withdraw_unchecked(deps, env, "execute_withdraw", denom, amount, address)
}

fn execute_update_allowlist(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    add_addresses: Vec<String>,
    remove_addresses: Vec<String>,
) -> Result<Response, ContractError> {
    update_allowlist(deps, add_addresses, remove_addresses)?;
    Ok(Response::new().add_attribute(ATTR_ACTION, "execute_update_allowlist"))
}

/// Adds and remove entries from the allow list.
fn update_allowlist(
    deps: DepsMut,
    add_addresses: Vec<String>,
    remove_addresses: Vec<String>,
) -> Result<(), ContractError> {
    for addr in add_addresses {
        let addr = deps.api.addr_validate(addr.as_str())?;
        ALLOWLIST.save(deps.storage, &addr, &ALLOWLIST_MARKER)?;
    }

    for addr in remove_addresses {
        let addr = deps.api.addr_validate(addr.as_str())?;
        ALLOWLIST.remove(deps.storage, &addr);
    }
    Ok(())
}

#[cfg_attr(not(feature = "library"), ::cosmwasm_std::entry_point)]
#[allow(unused)]
pub fn sudo(deps: DepsMut, env: Env, msg: SudoMsg) -> Result<Response, ContractError> {
    match msg {
        #[cfg(feature = "governance_owned")]
        SudoMsg::Withdraw {
            denom,
            amount,
            address,
        } => sudo_withdraw(deps, env, denom, amount, address),
        #[cfg(feature = "governance_owned")]
        SudoMsg::WithdrawToCommunityPool { denom, amount } => {
            sudo_withdraw_to_community_pool(deps, env, denom, amount)
        }
        #[cfg(feature = "governance_owned")]
        SudoMsg::SetConfig {
            manager,
            prices,
            payment,
            nois_beacon_price,
            callback_gas_limit,
            mode,
            allowlist_enabled,
            min_after,
            max_after,
        } => sudo_set_config(
            deps,
            env,
            manager,
            prices,
            payment,
            nois_beacon_price,
            callback_gas_limit,
            mode,
            allowlist_enabled,
            min_after,
            max_after,
        ),
    }
}

#[cfg(feature = "governance_owned")]
fn sudo_withdraw(
    deps: DepsMut,
    env: Env,
    denom: String,
    amount: Option<Uint128>,
    address: String,
) -> Result<Response, ContractError> {
    withdraw_unchecked(deps, env, "sudo_withdraw", denom, amount, address)
}

#[cfg(feature = "governance_owned")]
fn sudo_withdraw_to_community_pool(
    deps: DepsMut,
    env: Env,
    denom: String,
    amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    withdraw_to_community_pool_unchecked(
        deps,
        env,
        "sudo_withdraw_to_community_pool",
        denom,
        amount,
    )
}

#[cfg(feature = "governance_owned")]
#[allow(clippy::too_many_arguments)]
fn sudo_set_config(
    deps: DepsMut,
    env: Env,
    manager: Option<String>,
    prices: Option<Vec<Coin>>,
    payment: Option<String>,
    nois_beacon_price: Option<Uint128>,
    callback_gas_limit: Option<u64>,
    mode: Option<OperationalMode>,
    allowlist_enabled: Option<bool>,
    min_after: Option<Timestamp>,
    max_after: Option<Timestamp>,
) -> Result<Response, ContractError> {
    set_config_unchecked(
        deps,
        env,
        "sudo_set_config",
        manager,
        prices,
        payment,
        nois_beacon_price,
        callback_gas_limit,
        mode,
        allowlist_enabled,
        min_after,
        max_after,
    )
}

fn withdraw_unchecked(
    deps: DepsMut,
    env: Env,
    action: &str,
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
        .add_attribute(ATTR_ACTION, action)
        .add_attribute("amount", amount.to_string());
    Ok(res)
}

#[allow(unused)]
fn withdraw_to_community_pool_unchecked(
    deps: DepsMut,
    env: Env,
    action: &str,
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
        .add_attribute(ATTR_ACTION, action)
        .add_attribute("amount", amount.to_string());
    Ok(res)
}

#[allow(clippy::too_many_arguments)]
fn set_config_unchecked(
    deps: DepsMut,
    env: Env,
    action: &str,
    manager: Option<String>,
    prices: Option<Vec<Coin>>,
    payment: Option<String>,
    nois_beacon_price: Option<Uint128>,
    callback_gas_limit: Option<u64>,
    mode: Option<OperationalMode>,
    allowlist_enabled: Option<bool>,
    min_after: Option<Timestamp>,
    max_after: Option<Timestamp>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let manager = match manager {
        Some(ma) => Some(deps.api.addr_validate(&ma)?),
        None => config.manager,
    };
    let prices = prices.unwrap_or(config.prices);
    let test_mode = config.test_mode;
    let callback_gas_limit = callback_gas_limit.unwrap_or(config.callback_gas_limit);
    let payment = match payment {
        Some(pa) => Some(pa),
        None => config.payment,
    };
    let (nois_beacon_price, nois_beacon_price_updated) = match nois_beacon_price {
        Some(bp) => (bp, env.block.time),
        None => (config.nois_beacon_price, config.nois_beacon_price_updated),
    };
    let mode = mode.unwrap_or(config.mode);

    // Older versions of the proxy did not set allowlist_enabled (i.e. it was None/undefined in JSON).
    // This is normalized to Some(false) every time the value is used.
    let current_allowlist_enabled = config.allowlist_enabled.unwrap_or_default();
    let allowlist_enabled = allowlist_enabled.unwrap_or(current_allowlist_enabled);

    let min_after = match min_after {
        Some(new_value) => Some(new_value),
        None => config.min_after,
    };
    let max_after = match max_after {
        Some(new_value) => Some(new_value),
        None => config.max_after,
    };

    let new_config = Config {
        manager,
        prices,
        test_mode,
        callback_gas_limit,
        payment,
        nois_beacon_price,
        nois_beacon_price_updated,
        mode,
        allowlist_enabled: Some(allowlist_enabled),
        min_after,
        max_after,
    };

    CONFIG.save(deps.storage, &new_config)?;

    Ok(Response::default().add_attribute(ATTR_ACTION, action))
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
    let coin = Anybuf::new()
        .append_string(1, &amount.denom)
        .append_string(2, amount.amount.to_string());
    Anybuf::new()
        .append_message(1, &coin)
        .append_string(2, depositor)
        .into_vec()
}

#[cfg_attr(not(feature = "library"), ::cosmwasm_std::entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, reply: Reply) -> Result<Response, ContractError> {
    match reply.id {
        REPLAY_ID_CALLBACK => {
            let mut attributes = vec![];
            match reply.result {
                SubMsgResult::Ok(_) => {
                    attributes.push(Attribute::new(ATTR_CALLBACK_SUCCESS, "true"))
                }
                SubMsgResult::Err(err_msg) => {
                    attributes.push(Attribute::new(ATTR_CALLBACK_SUCCESS, "false"));
                    attributes.push(Attribute::new(ATTR_CALLBACK_ERROR_MSG, err_msg));
                }
            };
            let callback_event = Event::new(EVENT_TYPE_CALLBACK).add_attributes(attributes);
            Ok(Response::new().add_event(callback_event))
        }
        _ => Err(ContractError::UnknownReplyId { id: reply.id }),
    }
}

#[cfg_attr(not(feature = "library"), ::cosmwasm_std::entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<QueryResponse> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::Prices {} => to_binary(&query_prices(deps)?),
        QueryMsg::Price { denom } => to_binary(&query_price(deps, denom)?),
        QueryMsg::GatewayChannel {} => to_binary(&query_gateway_channel(deps)?),
        QueryMsg::Allowlist {} => to_binary(&query_allowlist(deps)?),
        QueryMsg::IsAllowlisted { address } => to_binary(&query_is_allowlisted(deps, address)?),
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

fn query_allowlist(deps: Deps) -> StdResult<AllowlistResponse> {
    // No pagination here yet 🤷‍♂️
    let allowed = ALLOWLIST
        .range(deps.storage, None, None, Order::Ascending)
        .map(|result| {
            let (address, _) = result.unwrap();
            address.into()
        })
        .collect();
    Ok(AllowlistResponse { allowed })
}

fn query_is_allowlisted(deps: Deps, addr: String) -> StdResult<IsAllowlistedResponse> {
    let addr = deps.api.addr_validate(&addr)?;
    Ok(IsAllowlistedResponse {
        listed: ALLOWLIST.has(deps.storage, &addr),
    })
}

#[cfg_attr(not(feature = "library"), ::cosmwasm_std::entry_point)]
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

#[cfg_attr(not(feature = "library"), ::cosmwasm_std::entry_point)]
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

    if GATEWAY_CHANNEL.exists(deps.storage) {
        return Err(ContractError::ChannelAlreadySet);
    }

    GATEWAY_CHANNEL.save(deps.storage, &channel_id)?;
    Ok(IbcBasicResponse::new()
        .add_attribute(ATTR_ACTION, "ibc_channel_connect")
        .add_attribute("channel_id", channel_id))
}

#[cfg_attr(not(feature = "library"), ::cosmwasm_std::entry_point)]
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
                .add_attribute(ATTR_ACTION, "ibc_channel_close")
                .add_attribute("channel_id", channel.endpoint.channel_id))
        }
    }
}

#[cfg_attr(not(feature = "library"), ::cosmwasm_std::entry_point)]
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
        REPLAY_ID_CALLBACK,
    )
    .with_gas_limit(callback_gas_limit);

    let ack = StdAck::success(to_binary(&OutPacketAck::DeliverBeacon {})?);
    Ok(IbcReceiveResponse::new()
        .set_ack(ack)
        .add_attribute(ATTR_ACTION, "receive_deliver_beacon")
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
    let ack = StdAck::success(to_binary(&OutPacketAck::Welcome {})?);
    Ok(IbcReceiveResponse::new()
        .set_ack(ack)
        .add_attribute(ATTR_ACTION, "receive_welcome"))
}

fn receive_push_beacon_price(
    deps: DepsMut,
    _env: Env,
    timestamp: Timestamp,
    amount: Uint128,
    denom: String,
) -> Result<IbcReceiveResponse, ContractError> {
    update_nois_beacon_price(deps, timestamp, amount, denom)?;
    let ack = StdAck::success(to_binary(&OutPacketAck::PushBeaconPrice {})?);
    Ok(IbcReceiveResponse::new()
        .set_ack(ack)
        .add_attribute(ATTR_ACTION, "receive_push_beacon_price"))
}

#[cfg_attr(not(feature = "library"), ::cosmwasm_std::entry_point)]
pub fn ibc_packet_ack(
    deps: DepsMut,
    _env: Env,
    msg: IbcPacketAckMsg,
) -> Result<IbcBasicResponse, ContractError> {
    let mut attributes = Vec::<Attribute>::new();
    attributes.push(attr(ATTR_ACTION, "ibc_packet_ack"));
    let ack: StdAck = from_binary(&msg.acknowledgement.data)?;
    let is_error: bool;
    match ack {
        StdAck::Success(data) => {
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

#[cfg_attr(not(feature = "library"), ::cosmwasm_std::entry_point)]
/// we just ignore these now. shall we store some info?
pub fn ibc_packet_timeout(
    _deps: DepsMut,
    _env: Env,
    _msg: IbcPacketTimeoutMsg,
) -> StdResult<IbcBasicResponse> {
    Ok(IbcBasicResponse::new().add_attribute(ATTR_ACTION, "ibc_packet_timeout"))
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

    fn setup(
        instantiate_msg: Option<InstantiateMsg>,
    ) -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
        let initial_funds = vec![
            Coin::new(22334455, "unoisx"),
            Coin::new(
                123321,
                "ibc/CB480EB3697F39DB828D9EFA021ABE681BFCD72E23894019B8DDB1AB94039081",
            ),
        ];
        let mut deps = mock_dependencies_with_balance(&initial_funds);
        let msg = instantiate_msg.unwrap_or_else(|| InstantiateMsg {
            manager: Some(CREATOR.to_string()),
            prices: vec![Coin::new(1_000000, "unoisx")],
            test_mode: Some(true),
            callback_gas_limit: 500_000,
            mode: OperationalMode::Funded {},
            allowlist_enabled: None,
            allowlist: None,
        });
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
            test_mode: None,
            callback_gas_limit: 500_000,
            mode: OperationalMode::Funded {},
            allowlist_enabled: None,
            allowlist: None,
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
        let mut deps = setup(None);

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
    fn get_next_randomness_for_invalid_inputs() {
        let mut deps = setup(None);
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
    fn get_next_randomness_for_allowed_address() {
        let mut deps = setup(Some(InstantiateMsg {
            manager: Some(CREATOR.to_string()),
            prices: vec![Coin::new(1_000000, "unoisx")],
            test_mode: Some(true),
            callback_gas_limit: 500_000,
            mode: OperationalMode::Funded {},
            allowlist_enabled: Some(true),
            allowlist: Some(vec![CREATOR.to_string()]),
        }));
        setup_channel(deps.as_mut());

        // Sender in allowlist
        let msg = ExecuteMsg::GetNextRandomness { job_id: "1".into() };
        let res = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(CREATOR, &coins(22334455, "unoisx")),
            msg,
        )
        .unwrap();
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
    fn get_next_randomness_for_disallowed_address() {
        let mut deps = setup(Some(InstantiateMsg {
            manager: Some(CREATOR.to_string()),
            prices: vec![Coin::new(1_000000, "unoisx")],
            test_mode: Some(true),
            callback_gas_limit: 500_000,
            mode: OperationalMode::Funded {},
            allowlist_enabled: Some(true),
            allowlist: Some(vec![CREATOR.to_string()]),
        }));
        setup_channel(deps.as_mut());

        // Sender in allowlist
        let msg = ExecuteMsg::GetNextRandomness { job_id: "1".into() };
        let err = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("disallowed", &coins(22334455, "unoisx")),
            msg,
        );
        assert!(matches!(err, Err(ContractError::SenderNotAllowed)));
    }

    #[test]
    fn get_randomness_after_works() {
        let mut deps = setup(None);

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
        let mut deps = setup(None);
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
    fn get_randomness_after_fails_for_after_values_out_of_range() {
        let mut deps = setup(None);

        // Requires a channel to forward requests to
        setup_channel(deps.as_mut());

        let instantiate_time = mock_env().block.time;

        // after == instantiate_time works
        let msg = ExecuteMsg::GetRandomnessAfter {
            after: instantiate_time,
            job_id: "foo".to_string(),
        };
        let info = mock_info("dapp", &coins(22334455, "unoisx"));
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // after < instantiate_time fails
        let msg = ExecuteMsg::GetRandomnessAfter {
            after: instantiate_time.minus_nanos(1),
            job_id: "foo".to_string(),
        };
        let info = mock_info("dapp", &coins(22334455, "unoisx"));
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        match err {
            ContractError::AfterTooLow { min_after, after } => {
                assert_eq!(min_after, instantiate_time);
                assert_eq!(after, instantiate_time.minus_nanos(1));
            }
            err => panic!("Unexpected error: {:?}", err),
        }

        // after has one 0 too much
        let msg = ExecuteMsg::GetRandomnessAfter {
            after: Timestamp::from_nanos(15717974198793055330),
            job_id: "foo".to_string(),
        };
        let info = mock_info("dapp", &coins(22334455, "unoisx"));
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        match err {
            ContractError::AfterTooHigh { max_after, after } => {
                assert_eq!(max_after, Timestamp::from_nanos(1887157419879305533));
                assert_eq!(after, Timestamp::from_nanos(15717974198793055330));
            }
            err => panic!("Unexpected error: {:?}", err),
        }
    }

    #[test]
    fn set_config_works() {
        let mut deps = setup(None);

        // Check original config
        let ConfigResponse { config: original } =
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
        assert_eq!(original.manager, Some(Addr::unchecked(CREATOR)));
        assert_eq!(original.callback_gas_limit, 500_000);
        assert_eq!(original.allowlist_enabled, Some(false));

        // Update nothing
        let msg = ExecuteMsg::SetConfig {
            manager: None,
            prices: None,
            payment: None,
            nois_beacon_price: None,
            callback_gas_limit: None,
            mode: None,
            allowlist_enabled: None,
            min_after: None,
            max_after: None,
        };
        execute(deps.as_mut(), mock_env(), mock_info(CREATOR, &[]), msg).unwrap();
        let ConfigResponse { config } =
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
        assert_eq!(config, original);

        // Set allowlist_enabled to true
        let msg = ExecuteMsg::SetConfig {
            manager: None,
            prices: None,
            payment: None,
            nois_beacon_price: None,
            callback_gas_limit: None,
            mode: None,
            allowlist_enabled: Some(true),
            min_after: None,
            max_after: None,
        };
        execute(deps.as_mut(), mock_env(), mock_info(CREATOR, &[]), msg).unwrap();
        let ConfigResponse { config } =
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
        // Updated
        assert_eq!(config.allowlist_enabled, Some(true));
        // Rest unchanged
        assert_eq!(config.prices, original.prices);
        assert_eq!(config.manager, original.manager);
        assert_eq!(config.mode, original.mode);
        assert_eq!(config.payment, original.payment);

        // Set allowlist_enabled to false
        let msg = ExecuteMsg::SetConfig {
            manager: None,
            prices: None,
            payment: None,
            nois_beacon_price: None,
            callback_gas_limit: None,
            mode: None,
            allowlist_enabled: Some(false),
            min_after: None,
            max_after: None,
        };
        execute(deps.as_mut(), mock_env(), mock_info(CREATOR, &[]), msg).unwrap();
        let ConfigResponse { config } =
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
        // Updated
        assert_eq!(config.allowlist_enabled, Some(false));
        // Rest unchanged
        assert_eq!(config.prices, original.prices);
        assert_eq!(config.manager, original.manager);
        assert_eq!(config.mode, original.mode);
        assert_eq!(config.payment, original.payment);

        // Update callback_gas_limit
        let msg = ExecuteMsg::SetConfig {
            manager: None,
            prices: None,
            payment: None,
            nois_beacon_price: None,
            callback_gas_limit: Some(800_000),
            mode: None,
            allowlist_enabled: None,
            min_after: None,
            max_after: None,
        };
        execute(deps.as_mut(), mock_env(), mock_info(CREATOR, &[]), msg).unwrap();
        let ConfigResponse { config } =
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
        // Updated
        assert_eq!(config.callback_gas_limit, 800_000);
        // Rest unchanged
        assert_eq!(config.prices, original.prices);
        assert_eq!(config.manager, original.manager);
        assert_eq!(config.mode, original.mode);
        assert_eq!(config.payment, original.payment);
    }

    #[test]
    fn withdraw_works() {
        let mut deps = setup(None);

        let msg = ExecuteMsg::Withdraw {
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
        let mut deps = setup(None);

        let msg = ExecuteMsg::Withdraw {
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

    #[test]
    fn withdraw_when_manager_is_not_set_manager_permissions_are_unauthorised() {
        // Check that if manager not set, a random person cannot execute manager-like operations.
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            manager: None,
            prices: vec![Coin::new(1_000000, "unoisx")],
            test_mode: None,
            callback_gas_limit: 500_000,
            mode: OperationalMode::Funded {},
            allowlist_enabled: None,
            allowlist: None,
        };
        let info = mock_info(CREATOR, &[]);
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        // withdraw
        let msg = ExecuteMsg::Withdraw {
            denom: "unoisx".to_string(),
            amount: Some(Uint128::new(12)),
            address: "some-address".to_string(),
        };
        let err = execute(deps.as_mut(), mock_env(), mock_info("dapp", &[]), msg).unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized));
        // withdraw all
        let msg = ExecuteMsg::Withdraw {
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
            callback_gas_limit: None,
            mode: None,
            allowlist_enabled: Some(false),
            min_after: None,
            max_after: None,
        };

        let err = execute(deps.as_mut(), mock_env(), mock_info("dapp", &[]), msg).unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized));
    }

    #[test]
    fn update_allowlist_works() {
        let mut deps = setup(None);
        let msg = ExecuteMsg::UpdateAllowlist {
            add: vec!["aaa".to_owned(), "ccc".to_owned()],
            remove: vec!["aaa".to_owned(), "bbb".to_owned()],
        };

        execute(deps.as_mut(), mock_env(), mock_info("dapp", &[]), msg).unwrap();

        assert!(!ALLOWLIST.has(&deps.storage, &Addr::unchecked("bbb")));
        assert!(ALLOWLIST.has(&deps.storage, &Addr::unchecked("ccc")));

        // If an address is both added and removed, err on the side or removing it,
        // hence, here we check that "aaa" is indeed not found.
        assert!(!ALLOWLIST.has(&deps.storage, &Addr::unchecked("aaa")));
    }

    //
    // Query tests
    //

    #[test]
    fn query_prices_works() {
        let deps = setup(None);

        let PricesResponse { prices } =
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Prices {}).unwrap()).unwrap();
        assert_eq!(prices, coins(1000000, "unoisx"));
    }

    #[test]
    fn query_price_works() {
        let deps = setup(None);

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

    #[test]
    fn query_allowlist_works() {
        // some list
        let addr_in_allowlist = vec![String::from("addr2"), String::from("addr1")];
        let deps = setup(Some(InstantiateMsg {
            manager: Some(CREATOR.to_string()),
            prices: vec![Coin::new(1_000000, "unoisx")],
            test_mode: Some(true),
            callback_gas_limit: 500_000,
            mode: OperationalMode::Funded {},
            allowlist_enabled: Some(true),
            allowlist: Some(addr_in_allowlist),
        }));

        let AllowlistResponse { allowed } =
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Allowlist {}).unwrap())
                .unwrap();
        assert_eq!(allowed, ["addr1", "addr2"]);

        // empty list
        let deps = setup(Some(InstantiateMsg {
            manager: Some(CREATOR.to_string()),
            prices: vec![Coin::new(1_000000, "unoisx")],
            test_mode: Some(true),
            callback_gas_limit: 500_000,
            mode: OperationalMode::Funded {},
            allowlist_enabled: Some(true),
            allowlist: Some(vec![]),
        }));

        let AllowlistResponse { allowed } =
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Allowlist {}).unwrap())
                .unwrap();
        assert!(allowed.is_empty());
    }

    #[test]
    fn query_is_allowed_works_when_allowlist_enabled() {
        let addr_in_allowlist = String::from("addr1");
        let addr_not_in_allowlist = String::from("addr2");
        let deps = setup(Some(InstantiateMsg {
            manager: Some(CREATOR.to_string()),
            prices: vec![Coin::new(1_000000, "unoisx")],
            test_mode: Some(true),
            callback_gas_limit: 500_000,
            mode: OperationalMode::Funded {},
            allowlist_enabled: Some(true),
            allowlist: Some(vec![addr_in_allowlist.clone()]),
        }));

        // expect the address IN allow list to return true
        let IsAllowlistedResponse { listed } = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::IsAllowlisted {
                    address: addr_in_allowlist,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert!(listed);

        // expect the address NOT in allow list to return false
        let IsAllowlistedResponse { listed } = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::IsAllowlisted {
                    address: addr_not_in_allowlist,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert!(!listed);
    }

    #[test]
    fn query_is_allowed_works_when_allowlist_disabled() {
        let addr_in_allowlist = String::from("addr1");
        let addr_not_in_allowlist = String::from("addr2");
        let deps = setup(Some(InstantiateMsg {
            manager: Some(CREATOR.to_string()),
            prices: vec![Coin::new(1_000000, "unoisx")],
            test_mode: Some(true),
            callback_gas_limit: 500_000,
            mode: OperationalMode::Funded {},
            allowlist_enabled: Some(false),
            allowlist: Some(vec![addr_in_allowlist.clone()]),
        }));

        // expect the address IN allow list to return true
        let IsAllowlistedResponse { listed } = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::IsAllowlisted {
                    address: addr_in_allowlist,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert!(listed);

        // Expect the address NOT in allowlist to return false even if allowlist is not enabled.
        let IsAllowlistedResponse { listed } = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::IsAllowlisted {
                    address: addr_not_in_allowlist,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert!(!listed);
    }

    //
    // IBC tests
    //

    #[test]
    fn ibc_channel_open_checks_version_and_order() {
        let mut deps = setup(None);

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

        let mut deps = setup(None);

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
        let mut deps = setup(None);

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
        let mut deps = setup(None);

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
        let ack = StdAck::success(
            to_binary(&InPacketAck::RequestProcessed {
                source_id: "backend:123:456".to_string(),
            })
            .unwrap(),
        );
        let msg = mock_ibc_packet_ack(
            "channel-12",
            &packet,
            IbcAcknowledgement::encode_json(&ack).unwrap(),
        )
        .unwrap();
        let IbcBasicResponse { attributes, .. } =
            ibc_packet_ack(deps.as_mut(), mock_env(), msg).unwrap();
        assert_eq!(first_attr(&attributes, "action").unwrap(), "ibc_packet_ack");
        assert_eq!(first_attr(&attributes, "is_error").unwrap(), "false");
        assert_eq!(first_attr(&attributes, "error"), None);
        assert_eq!(
            first_attr(&attributes, "ack_type").unwrap(),
            "request_processed"
        );

        // Success ack (queued)
        let ack = StdAck::success(
            to_binary(&InPacketAck::RequestQueued {
                source_id: "backend:123:456".to_string(),
            })
            .unwrap(),
        );
        let msg = mock_ibc_packet_ack(
            "channel-12",
            &packet,
            IbcAcknowledgement::encode_json(&ack).unwrap(),
        )
        .unwrap();
        let IbcBasicResponse { attributes, .. } =
            ibc_packet_ack(deps.as_mut(), mock_env(), msg).unwrap();
        assert_eq!(first_attr(&attributes, "action").unwrap(), "ibc_packet_ack");
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
        assert_eq!(first_attr(&attributes, "action").unwrap(), "ibc_packet_ack");
        assert_eq!(first_attr(&attributes, "is_error").unwrap(), "true");
        assert_eq!(first_attr(&attributes, "error").unwrap(), "kaputt");
        assert_eq!(first_attr(&attributes, "ack_type"), None);
    }
}
