use cosmwasm_std::{
    attr, ensure, ensure_eq, entry_point, from_binary, instantiate2_address, to_binary, Addr,
    Attribute, Binary, CodeInfoResponse, Coin, Deps, DepsMut, Empty, Env, Event, HexBinary,
    Ibc3ChannelOpenResponse, IbcBasicResponse, IbcChannelCloseMsg, IbcChannelConnectMsg,
    IbcChannelOpenMsg, IbcChannelOpenResponse, IbcMsg, IbcPacketAckMsg, IbcPacketReceiveMsg,
    IbcPacketTimeoutMsg, IbcReceiveResponse, MessageInfo, Never, Order, QueryRequest,
    QueryResponse, Response, StdError, StdResult, SystemError, SystemResult, Timestamp, WasmMsg,
    WasmQuery,
};
use cw_storage_plus::Bound;
use nois_protocol::{
    check_order, check_version, InPacket, InPacketAck, OutPacket, OutPacketAck, StdAck,
    BEACON_PRICE_PACKET_LIFETIME, IBC_APP_VERSION, WELCOME_PACKET_LIFETIME,
};
use sha2::{Digest, Sha256};

use crate::attributes::ATTR_ACTION;
use crate::error::ContractError;
use crate::job_id::validate_origin;
use crate::msg::{
    ConfigResponse, CustomerResponse, CustomersResponse, DrandJobStatsResponse, ExecuteMsg,
    InstantiateMsg, JobsResponse, QueriedCustomer, QueryMsg, RequestsLogResponse,
};
use crate::request_router::{NewDrand, RequestRouter, RoutingReceipt};
use crate::state::{
    all_unprocessed_drand_jobs, get_processed_drand_jobs, requests_log_add, requests_log_asc,
    requests_log_desc, unprocessed_drand_jobs_len, Config, Customer, RequestLogEntry, CONFIG,
    CUSTOMERS,
};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let InstantiateMsg {
        price,
        manager,
        payment_code_id,
        payment_initial_funds,
        sink,
    } = msg;

    let manager = deps.api.addr_validate(&manager)?;
    let sink = deps.api.addr_validate(&sink)?;

    ensure_code_id_exists(deps.as_ref(), payment_code_id)?;

    let config = Config {
        drand: None,
        trusted_sources: None,
        manager,
        price,
        payment_code_id,
        payment_initial_funds,
        sink,
    };
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::default())
}

#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, _msg: Empty) -> StdResult<Response> {
    let mut config = CONFIG.load(deps.storage)?;

    // In cases where drand was set but trusted sources is not,
    // we migrate over the drand address to trusted sources.
    // Before 0.14, drand was used as the only trusted source. However
    // for the sake of modularity and testing, we split the two things.
    if config
        .trusted_sources
        .clone()
        .unwrap_or_default()
        .is_empty()
    {
        if let Some(drand) = config.drand.clone() {
            config.trusted_sources = Some(vec![drand]);
        }
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default().add_attribute(ATTR_ACTION, "migrate"))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AddVerifiedRound {
            round,
            randomness,
            is_verifying_tx,
        } => execute_add_verified_round(deps, env, info, round, randomness, is_verifying_tx),
        ExecuteMsg::SetConfig {
            manager,
            price,
            drand_addr,
            payment_initial_funds,
            trusted_sources,
        } => execute_set_config(
            deps,
            info,
            env,
            manager,
            price,
            drand_addr,
            trusted_sources,
            payment_initial_funds,
        ),
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<QueryResponse> {
    let response = match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?)?,
        QueryMsg::DrandJobStats { round } => to_binary(&query_drand_job_stats(deps, round)?)?,
        QueryMsg::Customer { channel_id } => to_binary(&query_customer(deps, channel_id)?)?,
        QueryMsg::Customers { start_after, limit } => {
            to_binary(&query_customers(deps, start_after, limit)?)?
        }
        QueryMsg::JobsAsc { offset, limit } => {
            to_binary(&query_jobs(deps, Order::Ascending, offset, limit)?)?
        }
        QueryMsg::JobsDesc { offset, limit } => {
            to_binary(&query_jobs(deps, Order::Descending, offset, limit)?)?
        }
        QueryMsg::RequestsLogAsc {
            channel_id,
            offset,
            limit,
        } => to_binary(&query_requests_asc(deps, channel_id, offset, limit)?)?,
        QueryMsg::RequestsLogDesc {
            channel_id,
            offset,
            limit,
        } => to_binary(&query_requests_desc(deps, channel_id, offset, limit)?)?,
    };
    Ok(response)
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config)
}

/// Query drand job stats by drand round
fn query_drand_job_stats(deps: Deps, round: u64) -> StdResult<DrandJobStatsResponse> {
    let unprocessed = unprocessed_drand_jobs_len(deps.storage, round)?;
    let processed = get_processed_drand_jobs(deps.storage, round)?;
    Ok(DrandJobStatsResponse {
        round,
        unprocessed,
        processed,
    })
}

fn query_customer(deps: Deps, channel_id: String) -> StdResult<CustomerResponse> {
    let customer = CUSTOMERS.may_load(deps.storage, &channel_id)?;
    Ok(CustomerResponse {
        customer: customer.map(|c| QueriedCustomer::new(channel_id, c)),
    })
}

fn query_customers(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<CustomersResponse> {
    let limit = limit.unwrap_or(50) as usize;

    let start_after = start_after.as_deref();
    let low_bound = start_after.map(Bound::exclusive);

    let customers: Vec<_> = CUSTOMERS
        .range(deps.storage, low_bound, None, Order::Ascending)
        .take(limit)
        .map(|c| c.map(|(channel_id, customer)| QueriedCustomer::new(channel_id, customer)))
        .collect::<Result<_, _>>()?;

    Ok(CustomersResponse { customers })
}

fn query_jobs(
    deps: Deps,
    order: Order,
    offset: Option<u32>,
    limit: Option<u32>,
) -> StdResult<JobsResponse> {
    let offset = offset.unwrap_or(0) as usize;
    let limit = limit.unwrap_or(50) as usize;
    let jobs: Vec<_> = all_unprocessed_drand_jobs(deps.storage, order, offset, limit)?;
    Ok(JobsResponse { jobs })
}

fn query_requests_asc(
    deps: Deps,
    channel_id: String,
    offset: Option<u32>,
    limit: Option<u32>,
) -> StdResult<RequestsLogResponse> {
    let offset = offset.unwrap_or(0) as usize;
    let limit = limit.unwrap_or(50) as usize;
    let requests: Vec<_> = requests_log_asc(deps.storage, &channel_id, offset, limit)?;
    Ok(RequestsLogResponse { requests })
}

fn query_requests_desc(
    deps: Deps,
    channel_id: String,
    offset: Option<u32>,
    limit: Option<u32>,
) -> StdResult<RequestsLogResponse> {
    let offset = offset.unwrap_or(0) as usize;
    let limit = limit.unwrap_or(50) as usize;
    let requests: Vec<_> = requests_log_desc(deps.storage, &channel_id, offset, limit)?;
    Ok(RequestsLogResponse { requests })
}

#[entry_point]
/// enforces ordering and versioing constraints
pub fn ibc_channel_open(
    _deps: DepsMut,
    _env: Env,
    msg: IbcChannelOpenMsg,
) -> Result<IbcChannelOpenResponse, ContractError> {
    let (channel, counterparty_version) = match msg {
        IbcChannelOpenMsg::OpenInit { .. } => return Err(ContractError::MustBeChainB),
        IbcChannelOpenMsg::OpenTry {
            channel,
            counterparty_version,
        } => (channel, counterparty_version),
    };

    check_order(&channel.order)?;
    // In ibcv3 we don't check the version string passed in the message
    // and only check the counterparty version.
    check_version(&counterparty_version)?;

    // We return the version we need (which could be different than the counterparty version)
    Ok(Some(Ibc3ChannelOpenResponse {
        version: IBC_APP_VERSION.to_string(),
    }))
}

#[entry_point]
pub fn ibc_channel_connect(
    deps: DepsMut,
    env: Env,
    msg: IbcChannelConnectMsg,
) -> Result<IbcBasicResponse, ContractError> {
    let channel = match msg {
        IbcChannelConnectMsg::OpenAck { .. } => return Err(ContractError::MustBeChainB),
        IbcChannelConnectMsg::OpenConfirm { channel, .. } => channel,
    };
    let chan_id = channel.endpoint.channel_id;

    let config = CONFIG.load(deps.storage)?;

    // Instantiate payment contract
    let creator = deps.api.addr_canonicalize(env.contract.address.as_str())?;
    let CodeInfoResponse { checksum, .. } =
        deps.querier.query_wasm_code_info(config.payment_code_id)?;
    let salt = hash_channel(&chan_id);
    #[allow(unused)]
    let address = instantiate2_address(&checksum, &creator, &salt)
        .map_err(|e| StdError::generic_err(format!("Cound not generate address: {}", e)))?;

    let address = {
        #[cfg(not(test))]
        {
            deps.api.addr_humanize(&address)?
        }
        #[cfg(test)]
        cosmwasm_std::Addr::unchecked("some payment address")
    };
    let customer = Customer {
        payment: address,
        requested_beacons: 0,
    };
    CUSTOMERS.save(deps.storage, &chan_id, &customer)?;

    let funds = if let Some(pif) = config.payment_initial_funds {
        vec![pif]
    } else {
        vec![]
    };
    let instantiate_payment = WasmMsg::Instantiate2 {
        admin: Some(env.contract.address.into()), // Only gateway can update the contracts it created
        code_id: config.payment_code_id,
        label: format!("For {chan_id}"),
        msg: to_binary(&nois_payment::msg::InstantiateMsg {
            sink: config.sink.into(),
        })?,
        funds,
        salt,
    };

    // Send Welcome and BeaconPrice to proxy
    let welcome = IbcMsg::SendPacket {
        channel_id: chan_id.clone(),
        data: to_binary(&OutPacket::Welcome {
            payment: customer.payment.into(),
        })?,
        timeout: env.block.time.plus_seconds(WELCOME_PACKET_LIFETIME).into(),
    };
    let beacon_price = IbcMsg::SendPacket {
        channel_id: chan_id.clone(),
        data: to_binary(&OutPacket::PushBeaconPrice {
            timestamp: env.block.time,
            amount: config.price.amount,
            denom: config.price.denom,
        })?,
        timeout: env
            .block
            .time
            .plus_seconds(BEACON_PRICE_PACKET_LIFETIME)
            .into(),
    };

    Ok(IbcBasicResponse::new()
        .add_message(instantiate_payment)
        .add_message(welcome)
        .add_message(beacon_price)
        .add_attribute("action", "ibc_connect")
        .add_attribute("channel_id", chan_id)
        .add_event(Event::new("ibc").add_attribute("channel", "connect")))
}

#[entry_point]
pub fn ibc_channel_close(
    _deps: DepsMut,
    _env: Env,
    msg: IbcChannelCloseMsg,
) -> StdResult<IbcBasicResponse> {
    let channel = msg.channel();
    // get contract address and remove lookup
    let channel_id = channel.endpoint.channel_id.as_str();

    Ok(IbcBasicResponse::new()
        .add_attribute("action", "ibc_close")
        .add_attribute("channel_id", channel_id))
}

#[entry_point]
pub fn ibc_packet_receive(
    deps: DepsMut,
    env: Env,
    msg: IbcPacketReceiveMsg,
) -> Result<IbcReceiveResponse, Never> {
    let IbcPacketReceiveMsg {
        packet, relayer, ..
    } = msg;
    // which local channel did this packet come on
    let channel_id = packet.dest.channel_id;

    // put this in a closure so we can convert all error responses into acknowledgements
    (|| {
        let op: InPacket = from_binary(&packet.data)?;
        match op {
            InPacket::RequestBeacon { after, origin } => {
                receive_request_beacon(deps, env, channel_id, relayer, after, origin)
            }
            InPacket::PullBeaconPrice {} => receive_pull_beacon_price(deps, env),
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

fn receive_request_beacon(
    mut deps: DepsMut,
    env: Env,
    channel_id: String,
    relayer: Addr,
    after: Timestamp,
    origin: Binary,
) -> Result<IbcReceiveResponse, ContractError> {
    validate_origin(&origin)?;

    let router = RequestRouter::new();
    let RoutingReceipt {
        queued,
        source_id,
        acknowledgement,
        mut msgs,
    } = router.route(
        deps.branch(),
        &env,
        channel_id.clone(),
        after,
        origin.clone(),
    )?;

    // Store request
    requests_log_add(
        deps.storage,
        &channel_id,
        &RequestLogEntry {
            origin,
            tx: (env.block.height, env.transaction.map(|ti| ti.index)),
            source_id,
            queued,
        },
    )?;

    // Pay time
    let mut customer = CUSTOMERS.load(deps.storage, &channel_id)?;
    customer.requested_beacons += 1;
    CUSTOMERS.save(deps.storage, &channel_id, &customer)?;

    let config = CONFIG.load(deps.storage)?;

    let Coin { amount, denom } = config.price;
    let amount_burn = amount.mul_floor((50u128, 100)); // 50%
    let amount_relayer = amount.mul_floor((5u128, 100)); // 5%
    let amount_rest = amount - amount_burn - amount_relayer; // 45%

    let msg = WasmMsg::Execute {
        contract_addr: customer.payment.into(),
        msg: to_binary(&nois_payment::msg::ExecuteMsg::Pay {
            burn: Coin::new(amount_burn.u128(), &denom),
            relayer: (relayer.into(), Coin::new(amount_relayer.u128(), &denom)),
            community_pool: Coin::new(amount_rest.u128(), denom),
        })?,
        funds: vec![],
    };
    msgs.push(msg.into());

    Ok(IbcReceiveResponse::new()
        .set_ack(acknowledgement)
        .add_messages(msgs)
        .add_attribute("action", "receive_request_beacon"))
}

fn receive_pull_beacon_price(deps: DepsMut, env: Env) -> Result<IbcReceiveResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let Coin { amount, denom } = config.price;
    let ack = StdAck::success(InPacketAck::PullBeaconPrice {
        timestamp: env.block.time,
        amount,
        denom,
    });
    Ok(IbcReceiveResponse::new()
        .set_ack(ack)
        .add_attribute("action", "receive_pull_beacon_price"))
}

#[entry_point]
pub fn ibc_packet_ack(
    _deps: DepsMut,
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
            let _response: OutPacketAck = from_binary(&data)?;
        }
        StdAck::Error(err) => {
            is_error = true;
            attributes.push(attr("error", err));
        }
    }
    attributes.push(attr("is_error", is_error.to_string()));
    Ok(IbcBasicResponse::new().add_attributes(attributes))
}

#[entry_point]
/// never should be called as we do not send packets
pub fn ibc_packet_timeout(
    _deps: DepsMut,
    _env: Env,
    _msg: IbcPacketTimeoutMsg,
) -> StdResult<IbcBasicResponse> {
    Ok(IbcBasicResponse::new().add_attribute("action", "ibc_packet_timeout"))
}

/// This method simulates how the drand contract will call the front-desk contract to inform
/// it when there are is a new round. Here the verification was done at a trusted source so
/// we only send the raw randomness.
fn execute_add_verified_round(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    round: u64,
    randomness: HexBinary,
    is_verifying_tx: bool,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    ensure!(
        config
            .trusted_sources
            .unwrap_or_default()
            .contains(&info.sender),
        ContractError::UnauthorizedAddVerifiedRound
    );

    let mut attributes = Vec::<Attribute>::new();
    let router = RequestRouter::new();
    let NewDrand {
        msgs,
        jobs_processed,
    } = router.new_drand(deps, env, round, &randomness, is_verifying_tx)?;
    attributes.push(Attribute::new("jobs_processed", jobs_processed.to_string()));

    Ok(Response::new()
        .add_messages(msgs)
        .add_attributes(attributes))
}

/// In order not to fall in the chicken egg problem where you need
/// to instantiate two or more contracts that need to be aware of each other
/// in a context where the contract addresses generration is not known
/// in advance, we set the contract address at a later stage after the
/// instantation and make sure it is immutable once set
#[allow(clippy::too_many_arguments)]
fn execute_set_config(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    manager: Option<String>,
    price: Option<Coin>,
    drand: Option<String>,
    trusted_sources: Option<Vec<String>>,
    payment_initial_funds: Option<Coin>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // check the calling address is the authorised multisig
    ensure_eq!(info.sender, config.manager, ContractError::Unauthorized);

    let manager = match manager {
        Some(ma) => deps.api.addr_validate(&ma)?,
        None => config.manager,
    };
    let drand = match drand {
        Some(dr) => Some(deps.api.addr_validate(&dr)?),
        None => config.drand,
    };
    let trusted_sources = match trusted_sources {
        Some(ts) => {
            let mut addrs = Vec::<Addr>::new();
            for ts in ts {
                addrs.push(deps.api.addr_validate(&ts)?);
            }
            addrs
        }
        None => config.trusted_sources.unwrap_or_default(),
    };
    let price = price.unwrap_or(config.price);
    let payment_initial_funds = match payment_initial_funds {
        Some(pif) => Some(pif),
        None => config.payment_initial_funds,
    };

    let new_config = Config {
        manager,
        drand,
        price,
        payment_code_id: config.payment_code_id, // Use a separate message for this in order to migrate all existing contracts too
        payment_initial_funds,
        sink: config.sink, // Make updatable?
        trusted_sources: Some(trusted_sources),
    };

    CONFIG.save(deps.storage, &new_config)?;

    Ok(Response::default())
}

fn ensure_code_id_exists(deps: Deps, code_id: u64) -> Result<(), ContractError> {
    let query = to_binary(&QueryRequest::<Empty>::Wasm(WasmQuery::CodeInfo {
        code_id,
    }))?;
    match deps.querier.raw_query(&query) {
        SystemResult::Ok(_) => Ok(()),
        SystemResult::Err(SystemError::NoSuchCode { code_id }) => {
            Err(ContractError::CodeIdDoesNotExist { code_id })
        }
        SystemResult::Err(system_err) => {
            Err(StdError::generic_err(format!("Querier system error: {}", system_err)).into())
        }
    }
}

fn hash_channel(channel_id: &str) -> Binary {
    let mut hasher = Sha256::new();
    hasher.update(channel_id.as_bytes());
    let salt: [u8; 32] = hasher.finalize().into();
    Binary::from(salt)
}

#[cfg(test)]
mod tests {
    use crate::msg::ExecuteMsg;

    use super::*;
    use cosmwasm_std::testing::{
        self, mock_env, mock_ibc_channel_close_init, mock_ibc_channel_connect_confirm,
        mock_ibc_channel_open_try, mock_ibc_packet_ack, mock_ibc_packet_recv, mock_info, MockApi,
        MockQuerier, MockStorage,
    };
    use cosmwasm_std::{
        coin, from_binary, Addr, Binary, CodeInfoResponse, Coin, ContractResult, CosmosMsg,
        IbcAcknowledgement, IbcMsg, OwnedDeps, QuerierResult, SystemError, SystemResult, Timestamp,
        WasmQuery,
    };
    use nois_protocol::{APP_ORDER, BAD_APP_ORDER};

    const CREATOR: &str = "creator";
    const MANAGER: &str = "boss";
    const MANAGER2: &str = "boss2";
    const PAYMENT: u64 = 33;
    const PAYMENT2: u64 = 37;
    const SINK: &str = "sink";

    // Consecutive timestamps for the rounds 810, 820, 830, 840
    const AFTER1: Timestamp = Timestamp::from_seconds(1677687627 - 1);
    const AFTER2: Timestamp = Timestamp::from_seconds(1677687657 - 1);
    const AFTER3: Timestamp = Timestamp::from_seconds(1677687687 - 1);
    const AFTER4: Timestamp = Timestamp::from_seconds(1677687717 - 1);
    const ROUND1: u64 = 810;
    const ROUND2: u64 = 820;
    const ROUND3: u64 = 830;
    const ROUND4: u64 = 840;

    fn payment_initial() -> Option<Coin> {
        Some(coin(2_000000, "unois"))
    }

    fn setup() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            manager: MANAGER.to_string(),
            price: Coin::new(1, "unois"),
            payment_code_id: PAYMENT,
            payment_initial_funds: payment_initial(),
            sink: SINK.to_string(),
        };
        let info = mock_info(CREATOR, &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        deps
    }

    fn make_add_verified_round_msg(round: u64, is_verifying_tx: bool) -> ExecuteMsg {
        match round {
            9 => ExecuteMsg::AddVerifiedRound {
                // curl -sS https://drand.cloudflare.com/public/9
                round: 9,

                randomness: HexBinary::from_hex(
                    "1b9acda1c43e333bcf02ddce634b18ff79803a904097a5896710c7ae798b47ab",
                )
                .unwrap(),
                is_verifying_tx,
            },
            810 => ExecuteMsg::AddVerifiedRound {
                // curl -sS https://drand.cloudflare.com/dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493/public/810
                round: 810,
                randomness: HexBinary::from_hex(
                    "192af38cb4e26fd9d15e8b4968fb3df137f3e6d9b4aeb04c7c5b6201091872cc",
                )
                .unwrap(),
                is_verifying_tx,
            },
            820 => ExecuteMsg::AddVerifiedRound {
                // curl -sS https://drand.cloudflare.com/dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493/public/820
                round: 820,
                randomness: HexBinary::from_hex(
                    "32f614c72e9a382540f6cdca5f4d58537ea11de9b692bcdef7b10e892690d233",
                )
                .unwrap(),
                is_verifying_tx,
            },
            830 => ExecuteMsg::AddVerifiedRound {
                // curl -sS https://drand.cloudflare.com/dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493/public/830
                round: 830,
                randomness: HexBinary::from_hex(
                    "9e8d112e4c9b66e17ca3cd78aca91e6c076a42917a03fe1fe837f7eaf2fa8b86",
                )
                .unwrap(),
                is_verifying_tx,
            },
            840 => ExecuteMsg::AddVerifiedRound {
                // curl -sS https://drand.cloudflare.com/dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493/public/840
                round: 840,
                randomness: HexBinary::from_hex(
                    "59b949f6455a6d7319232f8fe085cbba884727cccf79fa5239579078c0a19cd4",
                )
                .unwrap(),
                is_verifying_tx,
            },
            72785 => ExecuteMsg::AddVerifiedRound {
                // curl -sS https://drand.cloudflare.com/public/72785
                round: 72785,
                randomness: HexBinary::from_hex(
                    "650be14f6ffd7dcb67df9138c3b7d7d6bca455d0438fc81d3fbb24a4ee038f36",
                )
                .unwrap(),
                is_verifying_tx,
            },
            72786 => ExecuteMsg::AddVerifiedRound {
                // curl -sS https://drand.cloudflare.com/public/72786
                round: 72786,
                randomness: HexBinary::from_hex(
                    "0ed47e6ebc311192000df4469bb5a5a00445a9365e428d61c8c08d78dd1e51a8",
                )
                .unwrap(),
                is_verifying_tx,
            },
            72787 => ExecuteMsg::AddVerifiedRound {
                // curl -sS https://drand.cloudflare.com/public/72787
                round: 72787,
                randomness: HexBinary::from_hex(
                    "d4ea3e5e43bf510c1b086613a9e68257b317202dbe5aab1b9182b65f51f4b82c",
                )
                .unwrap(),
                is_verifying_tx,
            },
            2183668 => ExecuteMsg::AddVerifiedRound {
                // curl -sS https://drand.cloudflare.com/public/2183668
                round: 2183668,
                randomness: HexBinary::from_hex(
                    "3436462283a07e695c41854bb953e5964d8737e7e29745afe54a9f4897b6c319",
                )
                .unwrap(),
                is_verifying_tx,
            },
            2183669 => ExecuteMsg::AddVerifiedRound {
                // curl -sS https://drand.cloudflare.com/public/2183669
                round: 2183669,
                randomness: HexBinary::from_hex(
                    "408de94b8c7e1972b06a4ab7636eb1ba2a176022a30d018c3b55e89289d41149",
                )
                .unwrap(),
                is_verifying_tx,
            },
            2183670 => ExecuteMsg::AddVerifiedRound {
                // curl -sS https://drand.cloudflare.com/public/2183670
                round: 2183670,
                randomness: HexBinary::from_hex(
                    "e5f7ba655389eee248575dde70cb9f3293c9774c8538136a135601907158d957",
                )
                .unwrap(),
                is_verifying_tx,
            },
            2183671 => ExecuteMsg::AddVerifiedRound {
                // curl -sS https://drand.cloudflare.com/public/2183671
                round: 2183671,
                randomness: HexBinary::from_hex(
                    "324e2a196293b42806c12c7bbd1aeba8d5617942f152a16588223f905f60801a",
                )
                .unwrap(),
                is_verifying_tx,
            },
            _ => panic!("Test round {round} not set"),
        }
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

    /// Creates a testing origin
    fn origin(job: u32) -> Binary {
        format!("job {job}").into_bytes().into()
    }

    fn connect(mut deps: DepsMut, channel_id: &str) {
        let handshake_open = mock_ibc_channel_open_try(channel_id, APP_ORDER, IBC_APP_VERSION);
        // first we try to open with a valid handshake
        ibc_channel_open(deps.branch(), mock_env(), handshake_open).unwrap();

        // then we connect (with counter-party version set)
        let handshake_connect =
            mock_ibc_channel_connect_confirm(channel_id, APP_ORDER, IBC_APP_VERSION);
        let res = ibc_channel_connect(deps.branch(), mock_env(), handshake_connect).unwrap();
        assert_eq!(res.messages.len(), 3);
        assert!(matches!(res.messages[0].msg, CosmosMsg::Wasm(_)));
        assert!(matches!(
            res.messages[1].msg,
            CosmosMsg::Ibc(IbcMsg::SendPacket { .. })
        ));
        assert!(matches!(
            res.messages[2].msg,
            CosmosMsg::Ibc(IbcMsg::SendPacket { .. })
        ));
        assert_eq!(res.events.len(), 1);
        assert_eq!(
            res.events[0],
            Event::new("ibc").add_attribute("channel", "connect"),
        );
    }

    fn mock_dependencies() -> OwnedDeps<MockStorage, MockApi, MockQuerier, Empty> {
        let mut deps = testing::mock_dependencies();
        deps.querier
            .update_wasm(Box::from(|request: &WasmQuery| -> QuerierResult {
                match request {
                    WasmQuery::Smart { contract_addr, .. } => {
                        SystemResult::Err(SystemError::NoSuchContract {
                            addr: contract_addr.clone(),
                        })
                    }
                    WasmQuery::Raw { contract_addr, .. } => {
                        SystemResult::Err(SystemError::NoSuchContract {
                            addr: contract_addr.clone(),
                        })
                    }
                    WasmQuery::ContractInfo { contract_addr, .. } => {
                        SystemResult::Err(SystemError::NoSuchContract {
                            addr: contract_addr.clone(),
                        })
                    }
                    WasmQuery::CodeInfo { code_id, .. } => match *code_id {
                        PAYMENT => {
                            let mut resp = CodeInfoResponse::default();
                            resp.code_id = PAYMENT;
                            resp.creator = "whoever".to_string();
                            resp.checksum = HexBinary::from_hex(
                                "04b59c31429dcc5bdc58fb1ded3894797a0f0c324f5db40e1fa2c7812a300b83",
                            )
                            .unwrap();
                            SystemResult::Ok(ContractResult::Ok(to_binary(&resp).unwrap()))
                        }
                        PAYMENT2 => {
                            let mut resp = CodeInfoResponse::default();
                            resp.code_id = PAYMENT2;
                            resp.creator = "anotherone".to_string();
                            resp.checksum = HexBinary::from_hex(
                                "f9ed2a2e7c03937004a2079747e79e508288e721bfe63f441f3e1c397c55b88d",
                            )
                            .unwrap();
                            SystemResult::Ok(ContractResult::Ok(to_binary(&resp).unwrap()))
                        }
                        _ => SystemResult::Err(SystemError::NoSuchCode { code_id: *code_id }),
                    },
                    _ => panic!("Unsupported WasmQuery case in mock handler"),
                }
            }));
        deps
    }

    //
    // Instantiate tests
    //

    #[test]
    fn instantiate_works() {
        let mut deps = mock_dependencies();

        // Without payment_initial_funds

        let msg = InstantiateMsg {
            manager: MANAGER.to_string(),
            price: Coin::new(1, "unois"),
            payment_code_id: PAYMENT,
            payment_initial_funds: None,
            sink: SINK.to_string(),
        };
        let info = mock_info("creator", &[]);
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let config: ConfigResponse =
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
        assert_eq!(
            config,
            ConfigResponse {
                drand: None,
                trusted_sources: None,
                manager: Addr::unchecked(MANAGER),
                price: Coin::new(1, "unois"),
                payment_code_id: PAYMENT,
                payment_initial_funds: None,
                sink: Addr::unchecked(SINK),
            }
        );

        // With payment_initial_funds

        let msg = InstantiateMsg {
            manager: MANAGER.to_string(),
            price: Coin::new(1, "unois"),
            payment_code_id: PAYMENT,
            payment_initial_funds: payment_initial(),
            sink: SINK.to_string(),
        };
        let info = mock_info("creator", &[]);
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let config: ConfigResponse =
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
        assert_eq!(
            config,
            ConfigResponse {
                drand: None,
                trusted_sources: None,
                manager: Addr::unchecked(MANAGER),
                price: Coin::new(1, "unois"),
                payment_code_id: PAYMENT,
                payment_initial_funds: payment_initial(),
                sink: Addr::unchecked(SINK),
            }
        );
    }

    #[test]
    fn instantiate_with_non_existing_code_id_fails() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            manager: MANAGER.to_string(),
            price: Coin::new(1, "unois"),
            payment_code_id: 654321,
            payment_initial_funds: payment_initial(),
            sink: SINK.to_string(),
        };
        let info = mock_info("creator", &[]);
        let env = mock_env();
        let err = instantiate(deps.as_mut(), env, info, msg).unwrap_err();
        match err {
            ContractError::CodeIdDoesNotExist { code_id } => assert_eq!(code_id, 654321), // ok
            err => panic!("Unexpected error: {:?}", err),
        }
    }

    //
    // Execute tests
    //

    #[test]
    fn execute_set_config_works() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            manager: MANAGER.to_string(),
            price: Coin::new(1, "unois"),
            payment_code_id: PAYMENT,
            payment_initial_funds: None,
            sink: SINK.to_string(),
        };
        let info = mock_info("creator", &[]);
        let env = mock_env();
        instantiate(deps.as_mut(), env, info, msg).unwrap();

        let msg = ExecuteMsg::SetConfig {
            manager: Some(MANAGER2.to_string()),
            price: Some(Coin::new(123, "unois")),
            drand_addr: Some("somewhere".to_string()),
            trusted_sources: Some(vec!["somewhere".to_string(), MANAGER2.to_string()]),
            payment_initial_funds: Some(Coin::new(500, "unois")),
        };

        // Fails for incorrect manager
        let err = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(MANAGER2, &[]),
            msg.clone(),
        )
        .unwrap_err();
        match err {
            ContractError::Unauthorized => {}
            _ => panic!("Unexpected error: {err:?}"),
        }

        // Works for correct manager
        execute(deps.as_mut(), mock_env(), mock_info(MANAGER, &[]), msg).unwrap();

        let config: ConfigResponse =
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
        assert_eq!(
            config,
            ConfigResponse {
                manager: Addr::unchecked(MANAGER2),
                price: Coin::new(123, "unois"),
                drand: Some(Addr::unchecked("somewhere")),
                trusted_sources: Some(vec![
                    Addr::unchecked("somewhere"),
                    Addr::unchecked(MANAGER2)
                ]),
                payment_code_id: PAYMENT,
                payment_initial_funds: Some(Coin::new(500, "unois")),
                sink: Addr::unchecked(SINK),
            }
        )
    }

    #[test]
    fn add_round_verified_must_only_be_called_by_drand() {
        let mut deps = mock_dependencies();

        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            manager: MANAGER.to_string(),
            price: Coin::new(1, "unois"),
            payment_code_id: PAYMENT,
            payment_initial_funds: payment_initial(),
            sink: SINK.to_string(),
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        const ANON: &str = "anon";
        const DRAND: &str = "drand_verifier_7";

        // drand contract unset, i.e. noone can submit
        let msg = make_add_verified_round_msg(2183668, true);
        let err = execute(deps.as_mut(), mock_env(), mock_info(ANON, &[]), msg).unwrap_err();
        assert!(matches!(err, ContractError::UnauthorizedAddVerifiedRound));
        let msg = make_add_verified_round_msg(2183668, true);
        let err = execute(deps.as_mut(), mock_env(), mock_info(DRAND, &[]), msg).unwrap_err();
        assert!(matches!(err, ContractError::UnauthorizedAddVerifiedRound));

        // Set drand contract
        let msg = ExecuteMsg::SetConfig {
            manager: None,
            price: None,
            drand_addr: Some(DRAND.to_string()),
            trusted_sources: None,
            payment_initial_funds: None,
        };
        let _res = execute(deps.as_mut(), mock_env(), mock_info(MANAGER, &[]), msg).unwrap();

        // drand cannot add
        let msg = make_add_verified_round_msg(2183668, true);
        let err = execute(deps.as_mut(), mock_env(), mock_info(DRAND, &[]), msg).unwrap_err();
        assert!(matches!(err, ContractError::UnauthorizedAddVerifiedRound));

        // Set trusted sources
        let msg = ExecuteMsg::SetConfig {
            manager: None,
            price: None,
            drand_addr: None,
            trusted_sources: Some(vec![DRAND.to_string(), MANAGER2.to_string()]),
            payment_initial_funds: None,
        };
        let _res = execute(deps.as_mut(), mock_env(), mock_info(MANAGER, &[]), msg).unwrap();

        // Anon still cannot add round
        let msg = make_add_verified_round_msg(2183668, true);
        let err = execute(deps.as_mut(), mock_env(), mock_info(ANON, &[]), msg).unwrap_err();
        assert!(matches!(err, ContractError::UnauthorizedAddVerifiedRound));

        // But drand and manager2 can
        let msg = make_add_verified_round_msg(2183668, true);
        let _res = execute(deps.as_mut(), mock_env(), mock_info(DRAND, &[]), msg).unwrap();
        let msg = make_add_verified_round_msg(2183668, true);
        let _res = execute(deps.as_mut(), mock_env(), mock_info(MANAGER2, &[]), msg).unwrap();
    }

    #[test]
    fn add_round_verified_processes_jobs() {
        let mut deps = mock_dependencies();

        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            manager: MANAGER.to_string(),
            price: Coin::new(1, "unois"),
            payment_code_id: PAYMENT,
            payment_initial_funds: payment_initial(),
            sink: SINK.to_string(),
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        const DRAND: &str = "drand_verifier_7";

        // Set drand contract
        let msg = ExecuteMsg::SetConfig {
            manager: None,
            price: None,
            drand_addr: Some(DRAND.to_string()),
            trusted_sources: Some(vec![DRAND.to_string()]),
            payment_initial_funds: None,
        };
        let _res = execute(deps.as_mut(), mock_env(), mock_info(MANAGER, &[]), msg).unwrap();

        // Create one job
        let msg = mock_ibc_packet_recv(
            "foo",
            &InPacket::RequestBeacon {
                after: AFTER2,
                origin: origin(1),
            },
        )
        .unwrap();
        ibc_packet_receive(deps.as_mut(), mock_env(), msg).unwrap();

        // Previous round processes no job
        let msg = make_add_verified_round_msg(ROUND1, true);
        let res = execute(deps.as_mut(), mock_env(), mock_info(DRAND, &[]), msg).unwrap();
        assert_eq!(res.messages.len(), 0);
        let jobs_processed = first_attr(&res.attributes, "jobs_processed").unwrap();
        assert_eq!(jobs_processed, "0");

        // Process one job
        let msg = make_add_verified_round_msg(ROUND2, true);
        let res = execute(deps.as_mut(), mock_env(), mock_info(DRAND, &[]), msg).unwrap();
        assert_eq!(res.messages.len(), 1);
        assert_eq!(res.messages[0].gas_limit, None);
        assert!(matches!(
            res.messages[0].msg,
            CosmosMsg::Ibc(IbcMsg::SendPacket { .. })
        ));
        let jobs_processed = first_attr(&res.attributes, "jobs_processed").unwrap();
        assert_eq!(jobs_processed, "1");

        // Create 2 job
        for i in 0..2 {
            let msg = mock_ibc_packet_recv(
                "foo",
                &InPacket::RequestBeacon {
                    after: AFTER3,
                    origin: origin(i),
                },
            )
            .unwrap();
            ibc_packet_receive(deps.as_mut(), mock_env(), msg).unwrap();
        }

        // Process 2 jobs
        let msg = make_add_verified_round_msg(ROUND3, true);
        let res = execute(deps.as_mut(), mock_env(), mock_info(DRAND, &[]), msg).unwrap();
        assert_eq!(res.messages.len(), 2);
        assert_eq!(res.messages[0].gas_limit, None);
        assert_eq!(res.messages[1].gas_limit, None);
        assert!(matches!(
            res.messages[0].msg,
            CosmosMsg::Ibc(IbcMsg::SendPacket { .. })
        ));
        assert!(matches!(
            res.messages[1].msg,
            CosmosMsg::Ibc(IbcMsg::SendPacket { .. })
        ));
        let jobs_processed = first_attr(&res.attributes, "jobs_processed").unwrap();
        assert_eq!(jobs_processed, "2");

        // Create 21 job
        for i in 0..21 {
            let msg = mock_ibc_packet_recv(
                "foo",
                &InPacket::RequestBeacon {
                    after: AFTER4,
                    origin: origin(i),
                },
            )
            .unwrap();
            let rec = ibc_packet_receive(deps.as_mut(), mock_env(), msg).unwrap();
            dbg!(rec.acknowledgement);
        }

        // Process first 2 jobs
        let msg = make_add_verified_round_msg(ROUND4, true);
        let res = execute(deps.as_mut(), mock_env(), mock_info(DRAND, &[]), msg).unwrap();
        assert_eq!(res.messages.len(), 2);
        let jobs_processed = first_attr(&res.attributes, "jobs_processed").unwrap();
        assert_eq!(jobs_processed, "2");

        // Process next 2 jobs
        let msg = make_add_verified_round_msg(ROUND4, true);
        let res = execute(deps.as_mut(), mock_env(), mock_info(DRAND, &[]), msg).unwrap();
        assert_eq!(res.messages.len(), 2);
        let jobs_processed = first_attr(&res.attributes, "jobs_processed").unwrap();
        assert_eq!(jobs_processed, "2");

        // Process next 2 jobs
        let msg = make_add_verified_round_msg(ROUND4, true);
        let res = execute(deps.as_mut(), mock_env(), mock_info(DRAND, &[]), msg).unwrap();
        assert_eq!(res.messages.len(), 2);
        let jobs_processed = first_attr(&res.attributes, "jobs_processed").unwrap();
        assert_eq!(jobs_processed, "2");

        // Process next 14 jobs
        let msg = make_add_verified_round_msg(ROUND4, false);
        let res = execute(deps.as_mut(), mock_env(), mock_info(DRAND, &[]), msg).unwrap();
        assert_eq!(res.messages.len(), 14);
        let jobs_processed = first_attr(&res.attributes, "jobs_processed").unwrap();
        assert_eq!(jobs_processed, "14");

        // Process last 1 jobs
        let msg = make_add_verified_round_msg(ROUND4, false);
        let res = execute(deps.as_mut(), mock_env(), mock_info(DRAND, &[]), msg).unwrap();
        assert_eq!(res.messages.len(), 1);
        let jobs_processed = first_attr(&res.attributes, "jobs_processed").unwrap();
        assert_eq!(jobs_processed, "1");

        // No jobs left for later submissions
        let msg = make_add_verified_round_msg(ROUND4, true);
        let res = execute(deps.as_mut(), mock_env(), mock_info(DRAND, &[]), msg).unwrap();
        assert_eq!(res.messages.len(), 0);
        let jobs_processed = first_attr(&res.attributes, "jobs_processed").unwrap();
        assert_eq!(jobs_processed, "0");
    }

    //
    // Query tests
    //

    #[test]
    fn query_job_stats_works() {
        let mut deps = mock_dependencies();

        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            manager: MANAGER.to_string(),
            price: Coin::new(1, "unois"),
            payment_code_id: PAYMENT,
            payment_initial_funds: payment_initial(),
            sink: SINK.to_string(),
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        const DRAND: &str = "drand_verifier_7";

        // Set drand contract
        let msg = ExecuteMsg::SetConfig {
            manager: None,
            price: None,
            drand_addr: Some(DRAND.to_string()),
            trusted_sources: Some(vec![DRAND.to_string()]),
            payment_initial_funds: None,
        };
        let _res = execute(deps.as_mut(), mock_env(), mock_info(MANAGER, &[]), msg).unwrap();

        fn job_stats(deps: Deps, round: u64) -> DrandJobStatsResponse {
            from_binary(&query(deps, mock_env(), QueryMsg::DrandJobStats { round }).unwrap())
                .unwrap()
        }

        // No jobs by default
        assert_eq!(
            job_stats(deps.as_ref(), ROUND1),
            DrandJobStatsResponse {
                round: ROUND1,
                processed: 0,
                unprocessed: 0,
            }
        );

        // Create one job
        let msg = mock_ibc_packet_recv(
            "foo",
            &InPacket::RequestBeacon {
                after: AFTER1,
                origin: origin(1),
            },
        )
        .unwrap();
        ibc_packet_receive(deps.as_mut(), mock_env(), msg).unwrap();

        // One unprocessed job
        assert_eq!(
            job_stats(deps.as_ref(), ROUND1),
            DrandJobStatsResponse {
                round: ROUND1,
                processed: 0,
                unprocessed: 1,
            }
        );

        let msg = make_add_verified_round_msg(ROUND1, true);
        execute(deps.as_mut(), mock_env(), mock_info(DRAND, &[]), msg).unwrap();

        // 1 processed job, no unprocessed jobs
        assert_eq!(
            job_stats(deps.as_ref(), ROUND1),
            DrandJobStatsResponse {
                round: ROUND1,
                processed: 1,
                unprocessed: 0,
            }
        );

        // New job for existing round gets processed immediately
        let msg = mock_ibc_packet_recv(
            "foo",
            &InPacket::RequestBeacon {
                after: AFTER1,
                origin: origin(2),
            },
        )
        .unwrap();
        ibc_packet_receive(deps.as_mut(), mock_env(), msg).unwrap();

        // 2 processed job, no unprocessed jobs
        assert_eq!(
            job_stats(deps.as_ref(), ROUND1),
            DrandJobStatsResponse {
                round: ROUND1,
                processed: 2,
                unprocessed: 0,
            }
        );

        // Create 20 jobs
        for i in 0..20 {
            let msg = mock_ibc_packet_recv(
                "foo",
                &InPacket::RequestBeacon {
                    after: AFTER2,
                    origin: origin(i),
                },
            )
            .unwrap();
            ibc_packet_receive(deps.as_mut(), mock_env(), msg).unwrap();
        }

        // 20 unprocessed
        assert_eq!(
            job_stats(deps.as_ref(), ROUND2),
            DrandJobStatsResponse {
                round: ROUND2,
                processed: 0,
                unprocessed: 20,
            }
        );

        // process some
        let msg = make_add_verified_round_msg(ROUND2, true);
        execute(deps.as_mut(), mock_env(), mock_info(DRAND, &[]), msg).unwrap();

        // Some processed, rest unprocessed
        assert_eq!(
            job_stats(deps.as_ref(), ROUND2),
            DrandJobStatsResponse {
                round: ROUND2,
                processed: 2,
                unprocessed: 18,
            }
        );
    }

    #[test]
    fn query_requests_works() {
        let mut deps = mock_dependencies();

        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            manager: MANAGER.to_string(),
            price: Coin::new(1, "unois"),
            payment_code_id: PAYMENT,
            payment_initial_funds: payment_initial(),
            sink: SINK.to_string(),
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        const DRAND: &str = "drand_verifier_7";
        const CHANNEL: &str = "channel-321";

        // Set drand contract
        let msg = ExecuteMsg::SetConfig {
            manager: None,
            price: None,
            drand_addr: Some(DRAND.to_string()),
            trusted_sources: Some(vec![DRAND.to_string()]),
            payment_initial_funds: None,
        };
        let _res = execute(deps.as_mut(), mock_env(), mock_info(MANAGER, &[]), msg).unwrap();

        fn requests_asc(deps: Deps, channel_id: &str) -> RequestsLogResponse {
            from_binary(
                &query(
                    deps,
                    mock_env(),
                    QueryMsg::RequestsLogAsc {
                        channel_id: channel_id.to_string(),
                        offset: None,
                        limit: None,
                    },
                )
                .unwrap(),
            )
            .unwrap()
        }

        fn requests_desc(deps: Deps, channel_id: &str) -> RequestsLogResponse {
            from_binary(
                &query(
                    deps,
                    mock_env(),
                    QueryMsg::RequestsLogDesc {
                        channel_id: channel_id.to_string(),
                        offset: None,
                        limit: None,
                    },
                )
                .unwrap(),
            )
            .unwrap()
        }

        // No requests by default
        assert_eq!(
            requests_asc(deps.as_ref(), CHANNEL),
            RequestsLogResponse { requests: vec![] }
        );
        assert_eq!(
            requests_desc(deps.as_ref(), CHANNEL),
            RequestsLogResponse { requests: vec![] }
        );

        // Create one job
        let msg = mock_ibc_packet_recv(
            CHANNEL,
            &InPacket::RequestBeacon {
                after: AFTER1,
                origin: origin(1),
            },
        )
        .unwrap();
        let receive_env = mock_env();
        ibc_packet_receive(deps.as_mut(), receive_env.clone(), msg).unwrap();

        // One unprocessed job
        let expected_tx_1 = (
            receive_env.block.height,
            receive_env.transaction.map(|ti| ti.index),
        );
        assert_eq!(
            requests_asc(deps.as_ref(), CHANNEL),
            RequestsLogResponse {
                requests: vec![RequestLogEntry {
                    origin: origin(1),
                    queued: true,
                    source_id:
                        "drand:dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493:810"
                            .to_string(),
                    tx: expected_tx_1
                }]
            }
        );
        assert_eq!(
            requests_desc(deps.as_ref(), CHANNEL),
            RequestsLogResponse {
                requests: vec![RequestLogEntry {
                    origin: origin(1),
                    queued: true,
                    source_id:
                        "drand:dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493:810"
                            .to_string(),
                    tx: expected_tx_1
                }]
            }
        );

        // Create second job
        let msg = mock_ibc_packet_recv(
            CHANNEL,
            &InPacket::RequestBeacon {
                after: AFTER2,
                origin: origin(2),
            },
        )
        .unwrap();
        let mut receive_env = mock_env();
        receive_env.block.height += 1;
        ibc_packet_receive(deps.as_mut(), receive_env.clone(), msg).unwrap();

        // two results now
        let expected_tx_2 = (
            receive_env.block.height,
            receive_env.transaction.map(|ti| ti.index),
        );
        assert_eq!(
            requests_asc(deps.as_ref(), CHANNEL),
            RequestsLogResponse {
                requests: vec![RequestLogEntry {
                    origin: origin(1),
                    queued: true,
                    source_id:
                        "drand:dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493:810"
                            .to_string(),
                    tx: expected_tx_1
                }, RequestLogEntry {
                    origin: origin(2),
                    queued: true,
                    source_id:
                        "drand:dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493:820"
                            .to_string(),
                    tx: expected_tx_2
                }]
            }
        );
        assert_eq!(
            requests_desc(deps.as_ref(), CHANNEL),
            RequestsLogResponse {
                requests: vec![RequestLogEntry {
                    origin: origin(2),
                    queued: true,
                    source_id:
                        "drand:dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493:820"
                            .to_string(),
                    tx: expected_tx_2
                  }, RequestLogEntry {
                    origin: origin(1),
                    queued: true,
                    source_id:
                        "drand:dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493:810"
                            .to_string(),
                    tx: expected_tx_1
                }]
            }
        );
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
    fn proper_handshake_flow() {
        let mut deps = setup();
        let channel_id = "channel-1234";

        // first we try to open with a valid handshake
        let handshake_open = mock_ibc_channel_open_try(channel_id, APP_ORDER, IBC_APP_VERSION);
        ibc_channel_open(deps.as_mut(), mock_env(), handshake_open).unwrap();

        // then we connect (with counter-party version set)
        let handshake_connect =
            mock_ibc_channel_connect_confirm(channel_id, APP_ORDER, IBC_APP_VERSION);
        let _res = ibc_channel_connect(deps.as_mut(), mock_env(), handshake_connect).unwrap();
    }

    #[test]
    fn check_close_channel() {
        let mut deps = setup();

        let channel_id = "channel-123";
        let account = "acct-123";

        // register the channel
        connect(deps.as_mut(), channel_id);
        // assign it some funds
        let funds = vec![coin(123456, "uatom"), coin(7654321, "tgrd")];
        deps.querier.update_balance(account, funds);

        // close the channel
        let channel = mock_ibc_channel_close_init(channel_id, APP_ORDER, IBC_APP_VERSION);
        let _res = ibc_channel_close(deps.as_mut(), mock_env(), channel).unwrap();
    }

    #[test]
    fn ibc_packet_ack_works() {
        let mut deps = setup();

        // The gateway -> proxy packet we get the acknowledgement for
        let packet = OutPacket::DeliverBeacon {
            source_id: "backend:123:456".to_string(),
            published: Timestamp::from_seconds(1682086395),
            randomness: HexBinary::from_hex(
                "aabbccddaabbccddaabbccddaabbccddaabbccddaabbccddaabbccddaabbccdd",
            )
            .unwrap(),
            origin: origin(1),
        };

        // Success ack (delivered)
        let ack = StdAck::success(OutPacketAck::DeliverBeacon {});
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
    }

    #[test]
    fn query_customer_works() {
        let mut deps = setup();

        let channel_id = "channel-123";

        // register the channel
        connect(deps.as_mut(), channel_id);

        // Existing channel
        let CustomerResponse { customer: address } = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Customer {
                    channel_id: channel_id.to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            address,
            Some(QueriedCustomer {
                channel_id: channel_id.to_string(),
                payment: Addr::unchecked("some payment address"),
                requested_beacons: 0,
            })
        );

        // Non-Existing channel
        let CustomerResponse { customer: address } = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Customer {
                    channel_id: "channel-does-not-exist".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(address, None);
    }

    #[test]
    fn query_customer_requested_beacons_works() {
        let mut deps = setup();

        const DRAND: &str = "drand_verifier_7";
        const CHANNEL_ID: &str = "the-channel";

        // register the channel
        connect(deps.as_mut(), CHANNEL_ID);

        // Set drand contract
        let msg = ExecuteMsg::SetConfig {
            manager: None,
            price: None,
            drand_addr: Some(DRAND.to_string()),
            trusted_sources: Some(vec![DRAND.to_string()]),
            payment_initial_funds: None,
        };
        let _res = execute(deps.as_mut(), mock_env(), mock_info(MANAGER, &[]), msg).unwrap();

        fn customer(deps: Deps, channel_id: &str) -> QueriedCustomer {
            let res: CustomerResponse = from_binary(
                &query(
                    deps,
                    mock_env(),
                    QueryMsg::Customer {
                        channel_id: channel_id.to_string(),
                    },
                )
                .unwrap(),
            )
            .unwrap();
            res.customer.unwrap()
        }

        // No jobs by default
        assert_eq!(customer(deps.as_ref(), CHANNEL_ID).requested_beacons, 0);

        // Create one job
        let msg = mock_ibc_packet_recv(
            CHANNEL_ID,
            &InPacket::RequestBeacon {
                after: AFTER1,
                origin: origin(1),
            },
        )
        .unwrap();
        ibc_packet_receive(deps.as_mut(), mock_env(), msg).unwrap();

        // One requested beacon
        assert_eq!(customer(deps.as_ref(), CHANNEL_ID).requested_beacons, 1);

        let msg = make_add_verified_round_msg(ROUND1, true);
        execute(deps.as_mut(), mock_env(), mock_info(DRAND, &[]), msg).unwrap();

        // New job for existing round gets processed immediately
        let msg = mock_ibc_packet_recv(
            CHANNEL_ID,
            &InPacket::RequestBeacon {
                after: AFTER1,
                origin: origin(2),
            },
        )
        .unwrap();
        ibc_packet_receive(deps.as_mut(), mock_env(), msg).unwrap();

        // 2 requested beacons
        assert_eq!(customer(deps.as_ref(), CHANNEL_ID).requested_beacons, 2);

        // Create more 20 jobs
        for i in 0..20 {
            let msg = mock_ibc_packet_recv(
                CHANNEL_ID,
                &InPacket::RequestBeacon {
                    after: AFTER2,
                    origin: origin(i),
                },
            )
            .unwrap();
            ibc_packet_receive(deps.as_mut(), mock_env(), msg).unwrap();
        }
        assert_eq!(customer(deps.as_ref(), CHANNEL_ID).requested_beacons, 22);
    }

    #[test]
    fn query_customers_works() {
        let mut deps = setup();

        let channel_id = "channel-123";

        let CustomersResponse {
            customers: addresses,
        } = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Customers {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(addresses, vec![]);

        // register the channel
        connect(deps.as_mut(), channel_id);

        let CustomersResponse {
            customers: addresses,
        } = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Customers {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            addresses,
            vec![QueriedCustomer {
                channel_id: channel_id.to_string(),
                payment: Addr::unchecked("some payment address"),
                requested_beacons: 0,
            }]
        );

        let CustomersResponse {
            customers: addresses,
        } = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Customers {
                    start_after: Some(channel_id.to_string()),
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(addresses, vec![]);
    }
}
