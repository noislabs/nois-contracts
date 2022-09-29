use cosmwasm_std::{
    entry_point, from_binary, from_slice, to_binary, Addr, Attribute, BankMsg, Binary, Coin,
    CosmosMsg, Deps, DepsMut, Env, Event, HexBinary, Ibc3ChannelOpenResponse, IbcBasicResponse,
    IbcChannelCloseMsg, IbcChannelConnectMsg, IbcChannelOpenMsg, IbcChannelOpenResponse, IbcMsg,
    IbcPacketAckMsg, IbcPacketReceiveMsg, IbcPacketTimeoutMsg, IbcReceiveResponse, MessageInfo,
    Order, QueryResponse, Response, StdError, StdResult, Storage, Timestamp,
};
use cw_storage_plus::Bound;
use drand_verify::{derive_randomness, g1_from_fixed, verify};
use nois_protocol::{
    check_order, check_version, DeliverBeaconPacket, DeliverBeaconPacketAck, RequestBeaconPacket,
    RequestBeaconPacketAck, StdAck, IBC_APP_VERSION,
};

use crate::drand::{DRAND_CHAIN_HASH, DRAND_GENESIS, DRAND_MAINNET_PUBKEY, DRAND_ROUND_LENGTH};
use crate::error::ContractError;
use crate::msg::{
    BeaconResponse, BeaconsResponse, BotsResponse, ConfigResponse, ExecuteMsg, InstantiateMsg,
    QueryMsg, Submission, SubmissionsResponse,
};
use crate::state::{
    Bot, Config, Job, QueriedBeacon, StoredSubmission, VerifiedBeacon, BEACONS, BOTS, CONFIG,
    DRAND_JOBS, SUBMISSIONS, TEST_MODE_NEXT_ROUND,
};

// TODO: make configurable?
/// packets live one hour
pub const PACKET_LIFETIME: u64 = 60 * 60;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let config = Config {
        test_mode: msg.test_mode,
        bot_incentive_base_price: msg.bot_incentive_base_price,
        native_denom: msg.native_denom,
    };
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AddRound {
            round,
            previous_signature,
            signature,
        } => execute_add_round(deps, env, info, round, previous_signature, signature),
        ExecuteMsg::RegisterBot { moniker } => execute_register_bot(deps, env, info, moniker),
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<QueryResponse> {
    let response = match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?)?,
        QueryMsg::Bots {} => to_binary(&query_bots(deps)?)?,
        QueryMsg::Beacon { round } => to_binary(&query_beacon(deps, round)?)?,
        QueryMsg::BeaconsAsc { start_after, limit } => {
            to_binary(&query_beacons(deps, start_after, limit, Order::Ascending)?)?
        }
        QueryMsg::BeaconsDesc { start_after, limit } => {
            to_binary(&query_beacons(deps, start_after, limit, Order::Descending)?)?
        }
        QueryMsg::Submissions { round } => to_binary(&query_submissions(deps, round)?)?,
    };
    Ok(response)
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config)
}

fn query_bots(deps: Deps) -> StdResult<BotsResponse> {
    let bots = BOTS
        .prefix_range(deps.storage, None, None, Order::Ascending)
        .map(|bot| (bot.unwrap().1))
        .collect();
    Ok(BotsResponse { bots })
}

// Query beacon by round
fn query_beacon(deps: Deps, round: u64) -> StdResult<BeaconResponse> {
    let beacon = BEACONS.may_load(deps.storage, round)?;
    Ok(BeaconResponse {
        beacon: beacon.map(|b| QueriedBeacon::make(b, round)),
    })
}

fn query_beacons(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
    order: Order,
) -> StdResult<BeaconsResponse> {
    let limit: usize = limit.unwrap_or(100) as usize;
    let (low_bound, top_bound) = match order {
        Order::Ascending => (start_after.map(Bound::exclusive), None),
        Order::Descending => (None, start_after.map(Bound::exclusive)),
    };
    let beacons: Vec<QueriedBeacon> = BEACONS
        .range(deps.storage, low_bound, top_bound, order)
        .take(limit)
        .map(|c| c.map(|(round, beacon)| QueriedBeacon::make(beacon, round)))
        .collect::<Result<_, _>>()?;
    Ok(BeaconsResponse { beacons })
}

// Query submissions by round
fn query_submissions(deps: Deps, round: u64) -> StdResult<SubmissionsResponse> {
    let min_addr = Addr::unchecked("\0"); // NULL: lower than all printable ASCII
    let max_addr = Addr::unchecked("\x7f"); // DEL: larger than all printable ASCII
    let from = Some(Bound::inclusive((round, &min_addr)));
    let to = Some(Bound::exclusive((round, &max_addr)));
    let submissions = SUBMISSIONS.range(deps.storage, from, to, Order::Ascending);
    let submissions: Vec<Submission> = submissions
        .map(|item| {
            item.map(|((_round, bot), submission)| Submission {
                bot,
                time: submission.time,
            })
        })
        .collect::<Result<_, _>>()?;
    Ok(SubmissionsResponse { round, submissions })
}

#[entry_point]
/// enforces ordering and versioing constraints
pub fn ibc_channel_open(
    _deps: DepsMut,
    _env: Env,
    msg: IbcChannelOpenMsg,
) -> Result<IbcChannelOpenResponse, ContractError> {
    let channel = msg.channel();

    check_order(&channel.order)?;
    // In ibcv3 we don't check the version string passed in the message
    // and only check the counterparty version.
    if let Some(counter_version) = msg.counterparty_version() {
        check_version(counter_version)?;
    }

    // We return the version we need (which could be different than the counterparty version)
    Ok(Some(Ibc3ChannelOpenResponse {
        version: IBC_APP_VERSION.to_string(),
    }))
}

#[entry_point]
pub fn ibc_channel_connect(
    _deps: DepsMut,
    _env: Env,
    msg: IbcChannelConnectMsg,
) -> StdResult<IbcBasicResponse> {
    let channel = msg.channel();
    let chan_id = &channel.endpoint.channel_id;

    Ok(IbcBasicResponse::new()
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
) -> Result<IbcReceiveResponse, ContractError> {
    let packet = msg.packet;
    // which local channel did this packet come on
    let channel = packet.dest.channel_id;
    let msg: RequestBeaconPacket = from_slice(&packet.data)?;
    receive_get_beacon(deps, env, channel, msg.after, msg.sender, msg.job_id)
}

fn receive_get_beacon(
    deps: DepsMut,
    env: Env,
    channel: String,
    after: Timestamp,
    sender: String,
    job_id: String,
) -> Result<IbcReceiveResponse, ContractError> {
    let Config { test_mode, .. } = CONFIG.load(deps.storage)?;
    let mode = if test_mode {
        NextRoundMode::Test
    } else {
        NextRoundMode::Time { base: after }
    };
    let (round, source_id) = next_round(deps.storage, mode)?;

    let job = Job {
        source_id: source_id.clone(),
        channel,
        sender,
        job_id,
    };

    let beacon = BEACONS.may_load(deps.storage, round)?;

    let mut msgs = Vec::<CosmosMsg>::new();

    let acknowledgement: Binary = if let Some(beacon) = beacon.as_ref() {
        //If the drand round already exists we send it
        let msg = process_job(env.block.time, job, beacon)?;
        msgs.push(msg.into());
        StdAck::success(&RequestBeaconPacketAck::Processed { source_id })
    } else {
        // If we don't have the beacon yet we store the job for later
        let mut jobs = DRAND_JOBS
            .may_load(deps.storage, round)?
            .unwrap_or_default();
        jobs.push(job);
        DRAND_JOBS.save(deps.storage, round, &jobs)?;
        StdAck::success(&RequestBeaconPacketAck::Queued { source_id })
    };

    Ok(IbcReceiveResponse::new()
        .set_ack(acknowledgement)
        .add_messages(msgs)
        .add_attribute("action", "receive_get_beacon"))
}

fn process_job(
    blocktime: Timestamp,
    job: Job,
    beacon: &VerifiedBeacon,
) -> Result<IbcMsg, ContractError> {
    let packet = DeliverBeaconPacket {
        sender: job.sender,
        job_id: job.job_id,
        randomness: beacon.randomness.clone(),
        source_id: job.source_id,
    };
    let msg = IbcMsg::SendPacket {
        channel_id: job.channel,
        data: to_binary(&packet)?,
        timeout: blocktime.plus_seconds(PACKET_LIFETIME).into(),
    };
    Ok(msg)
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum NextRoundMode {
    Test,
    Time { base: Timestamp },
}

/// Calculates the next round in the future, i.e. publish time > base time.
fn next_round(storage: &mut dyn Storage, mode: NextRoundMode) -> StdResult<(u64, String)> {
    match mode {
        NextRoundMode::Test => {
            let next = TEST_MODE_NEXT_ROUND.may_load(storage)?.unwrap_or(2183660);
            TEST_MODE_NEXT_ROUND.save(storage, &(next + 1))?;
            let source_id = format!("test-mode:{}", next);
            Ok((next, source_id))
        }
        NextRoundMode::Time { base } => {
            // Losely ported from https://github.com/drand/drand/blob/eb36ba81e3f28c966f95bcd602f60e7ff8ef4c35/chain/time.go#L49-L63
            let round = if base < DRAND_GENESIS {
                1
            } else {
                let from_genesis = base.nanos() - DRAND_GENESIS.nanos();
                let periods_since_genesis = from_genesis / DRAND_ROUND_LENGTH;
                let next_period_index = periods_since_genesis + 1;
                next_period_index + 1 // Convert 0-based counting to 1-based counting
            };
            let source_id = format!("drand:{}:{}", DRAND_CHAIN_HASH, round);
            Ok((round, source_id))
        }
    }
}

#[entry_point]
pub fn ibc_packet_ack(
    _deps: DepsMut,
    _env: Env,
    msg: IbcPacketAckMsg,
) -> Result<IbcBasicResponse, ContractError> {
    let ack: StdAck = from_binary(&msg.acknowledgement.data)?;
    match ack {
        StdAck::Result(data) => {
            let _response: DeliverBeaconPacketAck = from_binary(&data)?;
            // alright
            Ok(IbcBasicResponse::new().add_attribute("action", "ibc_packet_ack"))
        }
        StdAck::Error(err) => Err(ContractError::ForeignError { err }),
    }
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

fn execute_register_bot(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    moniker: String,
) -> Result<Response, ContractError> {
    match BOTS.may_load(deps.storage, info.sender.to_owned())? {
        Some(mut bot) => {
            bot.moniker = moniker;
            BOTS.save(deps.storage, bot.address.to_owned(), &bot)?;
        }
        _ => {
            let bot = Bot {
                moniker: (moniker),
                address: (info.sender),
                number_of_added_rounds: (0),
            };
            BOTS.save(deps.storage, bot.address.to_owned(), &bot)?;
        }
    }
    Ok(Response::default())
}

fn execute_add_round(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    round: u64,
    previous_signature: HexBinary,
    signature: HexBinary,
) -> Result<Response, ContractError> {
    // Handle sender is not sending funds
    if !info.funds.is_empty() {
        return Err(StdError::generic_err("Do not send funds").into());
    }

    let pk = g1_from_fixed(DRAND_MAINNET_PUBKEY).map_err(|_| ContractError::InvalidPubkey {})?;
    let is_valid = verify(&pk, round, &previous_signature, &signature).unwrap_or(false);

    if !is_valid {
        return Err(ContractError::InvalidSignature {});
    }

    let randomness: HexBinary = derive_randomness(signature.as_slice()).into();

    let beacon = &VerifiedBeacon {
        verified: env.block.time,
        randomness: randomness.clone(),
    };

    let submissions_key = (round, &info.sender);

    if SUBMISSIONS.has(deps.storage, submissions_key) {
        return Err(ContractError::SubmissionExists);
    }

    let bot = BOTS.may_load(deps.storage, info.sender.to_owned())?;
    if bot.is_some() {
        bot.to_owned().unwrap().number_of_added_rounds += 1;
        BOTS.save(deps.storage, info.sender.to_owned(), &bot.unwrap())?;
    }

    SUBMISSIONS.save(
        deps.storage,
        submissions_key,
        &StoredSubmission {
            time: env.block.time,
        },
    )?;

    //Pay the bot incentive
    let denom = CONFIG.load(deps.storage)?.native_denom;
    let bot_incentive_base_price = CONFIG.load(deps.storage)?.bot_incentive_base_price;
    let contract_balance = deps
        .querier
        .query_balance(&env.contract.address, &denom)?
        .amount;
    let bot_desired_incentive =
        calculate_bot_incentive_coefficient() * bot_incentive_base_price.u128();
    let attributes = vec![
        Attribute::new("round", round.to_string()),
        Attribute::new("randomness", randomness.to_hex()),
        Attribute::new("worker", info.sender.to_string()),
        Attribute::new("bot_incentive", bot_desired_incentive.to_string()),
    ];

    let mut response = Response::new().add_attributes(attributes);
    if contract_balance > bot_desired_incentive.into()
        && BOTS.has(deps.storage, info.sender.to_owned())
    {
        //Bot registered and there's enough funds in the contract
        let bot_incentive_msg = BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![Coin::new(bot_desired_incentive, denom)],
        };
        response = response.add_message(bot_incentive_msg);
    }

    if !BEACONS.has(deps.storage, round) {
        // Round is new
        BEACONS.save(deps.storage, round, beacon)?;

        let mut msgs = Vec::<CosmosMsg>::new();
        if let Some(jobs) = DRAND_JOBS.may_load(deps.storage, round)? {
            DRAND_JOBS.remove(deps.storage, round);

            for job in jobs {
                // Use IbcMsg::SendPacket to send packages to the proxies.
                let msg = process_job(env.block.time, job, beacon)?;
                msgs.push(msg.into());
            }
        }

        Ok(response.add_messages(msgs))
    } else {
        // Round has already been verified and must not be overriden to not
        // get a wrong `verified` timestamp.
        Ok(response)
    }
}

fn calculate_bot_incentive_coefficient() -> u128 {
    1 //For now we just incentivise with the base/minimum price. We need to implement here the incentive logic and who gets how much
}
