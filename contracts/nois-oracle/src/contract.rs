use cosmwasm_std::{
    entry_point, from_binary, from_slice, to_binary, Addr, Attribute, Binary, CosmosMsg, Deps,
    DepsMut, Env, Event, HexBinary, Ibc3ChannelOpenResponse, IbcBasicResponse, IbcChannelCloseMsg,
    IbcChannelConnectMsg, IbcChannelOpenMsg, IbcChannelOpenResponse, IbcMsg, IbcPacketAckMsg,
    IbcPacketReceiveMsg, IbcPacketTimeoutMsg, IbcReceiveResponse, MessageInfo, Order,
    QueryResponse, Response, StdError, StdResult, Storage, Timestamp,
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
    BeaconResponse, BeaconsResponse, ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg,
    Submission, SubmissionsResponse,
};
use crate::state::{
    Config, Job, QueriedBeacon, StoredSubmission, VerifiedBeacon, BEACONS, CONFIG, DRAND_JOBS,
    SUBMISSIONS, TEST_MODE_NEXT_ROUND,
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
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<QueryResponse> {
    let response = match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?)?,
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

    let attributes = vec![
        Attribute::new("round", round.to_string()),
        Attribute::new("randomness", randomness.to_hex()),
        Attribute::new("worker", info.sender.to_string()),
    ];

    let submissions_key = (round, &info.sender);

    if SUBMISSIONS.has(deps.storage, submissions_key) {
        return Err(ContractError::SubmissionExists);
    }
    SUBMISSIONS.save(
        deps.storage,
        submissions_key,
        &StoredSubmission {
            time: env.block.time,
        },
    )?;

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
        Ok(Response::new()
            .add_messages(msgs)
            .add_attributes(attributes))
    } else {
        // Round has already been verified and must not be overriden to not
        // get a wrong `verified` timestamp.
        Ok(Response::new().add_attributes(attributes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{
        mock_dependencies, mock_env, mock_ibc_channel_close_init, mock_ibc_channel_connect_ack,
        mock_ibc_channel_open_init, mock_ibc_channel_open_try, mock_info, MockApi, MockQuerier,
        MockStorage,
    };
    use cosmwasm_std::{coin, from_binary, OwnedDeps};
    use nois_protocol::{APP_ORDER, BAD_APP_ORDER};

    const CREATOR: &str = "creator";

    fn setup() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg { test_mode: true };
        let info = mock_info(CREATOR, &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        deps
    }

    // connect will run through the entire handshake to set up a proper connect and
    // save the account (tested in detail in `proper_handshake_flow`)
    fn connect(mut deps: DepsMut, channel_id: &str, account: impl Into<String>) {
        let _account: String = account.into();

        let handshake_open = mock_ibc_channel_open_init(channel_id, APP_ORDER, IBC_APP_VERSION);
        // first we try to open with a valid handshake
        ibc_channel_open(deps.branch(), mock_env(), handshake_open).unwrap();

        // then we connect (with counter-party version set)
        let handshake_connect =
            mock_ibc_channel_connect_ack(channel_id, APP_ORDER, IBC_APP_VERSION);
        let res = ibc_channel_connect(deps.branch(), mock_env(), handshake_connect).unwrap();
        assert_eq!(res.messages.len(), 0);
        assert_eq!(res.events.len(), 1);
        assert_eq!(
            res.events[0],
            Event::new("ibc").add_attribute("channel", "connect"),
        );
    }

    //
    // Instantiate tests
    //

    #[test]
    fn instantiate_works() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg { test_mode: true };
        let info = mock_info("creator", &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    //
    // Execute tests
    //

    #[test]
    fn add_round_verifies_and_stores_randomness() {
        let mut deps = mock_dependencies();

        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg { test_mode: true };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("anyone", &[]);
        let msg = ExecuteMsg::AddRound {
            // curl -sS https://drand.cloudflare.com/public/72785
            round: 72785,
            previous_signature: HexBinary::from_hex("a609e19a03c2fcc559e8dae14900aaefe517cb55c840f6e69bc8e4f66c8d18e8a609685d9917efbfb0c37f058c2de88f13d297c7e19e0ab24813079efe57a182554ff054c7638153f9b26a60e7111f71a0ff63d9571704905d3ca6df0b031747").unwrap(),
            signature: HexBinary::from_hex("82f5d3d2de4db19d40a6980e8aa37842a0e55d1df06bd68bddc8d60002e8e959eb9cfa368b3c1b77d18f02a54fe047b80f0989315f83b12a74fd8679c4f12aae86eaf6ab5690b34f1fddd50ee3cc6f6cdf59e95526d5a5d82aaa84fa6f181e42").unwrap(),
        };
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let response: BeaconResponse = from_binary(
            &query(deps.as_ref(), mock_env(), QueryMsg::Beacon { round: 72785 }).unwrap(),
        )
        .unwrap();
        assert_eq!(
            response.beacon.unwrap().randomness.to_hex(),
            "8b676484b5fb1f37f9ec5c413d7d29883504e5b669f604a1ce68b3388e9ae3d9"
        );
    }

    #[test]
    fn add_round_fails_for_broken_signature() {
        let mut deps = mock_dependencies();

        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg { test_mode: true };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("anyone", &[]);
        let msg = ExecuteMsg::AddRound {
            // curl -sS https://drand.cloudflare.com/public/72785
            round: 72785,
            previous_signature: hex::decode("a609e19a03c2fcc559e8dae14900aaefe517cb55c840f6e69bc8e4f66c8d18e8a609685d9917efbfb0c37f058c2de88f13d297c7e19e0ab24813079efe57a182554ff054c7638153f9b26a60e7111f71a0ff63d9571704905d3ca6df0b031747").unwrap().into(),
            signature: hex::decode("3cc6f6cdf59e95526d5a5d82aaa84fa6f181e4").unwrap().into(), // broken signature
        };
        let result = execute(deps.as_mut(), mock_env(), info, msg);
        match result.unwrap_err() {
            ContractError::InvalidSignature {} => {}
            err => panic!("Unexpected error: {:?}", err),
        };
    }

    #[test]
    fn add_round_fails_for_invalid_signature() {
        let mut deps = mock_dependencies();

        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg { test_mode: true };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("anyone", &[]);
        let msg = ExecuteMsg::AddRound {
            // curl -sS https://drand.cloudflare.com/public/72785
            round: 1111, // wrong round
            previous_signature: hex::decode("a609e19a03c2fcc559e8dae14900aaefe517cb55c840f6e69bc8e4f66c8d18e8a609685d9917efbfb0c37f058c2de88f13d297c7e19e0ab24813079efe57a182554ff054c7638153f9b26a60e7111f71a0ff63d9571704905d3ca6df0b031747").unwrap().into(),
            signature: hex::decode("82f5d3d2de4db19d40a6980e8aa37842a0e55d1df06bd68bddc8d60002e8e959eb9cfa368b3c1b77d18f02a54fe047b80f0989315f83b12a74fd8679c4f12aae86eaf6ab5690b34f1fddd50ee3cc6f6cdf59e95526d5a5d82aaa84fa6f181e42").unwrap().into(),
        };
        let result = execute(deps.as_mut(), mock_env(), info, msg);
        match result.unwrap_err() {
            ContractError::InvalidSignature {} => {}
            err => panic!("Unexpected error: {:?}", err),
        };
    }

    #[test]
    fn add_round_succeeds_multiple_times() {
        let mut deps = mock_dependencies();

        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg { test_mode: true };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg = ExecuteMsg::AddRound {
            // curl -sS https://drand.cloudflare.com/public/72785
            round: 72785,
            previous_signature: HexBinary::from_hex("a609e19a03c2fcc559e8dae14900aaefe517cb55c840f6e69bc8e4f66c8d18e8a609685d9917efbfb0c37f058c2de88f13d297c7e19e0ab24813079efe57a182554ff054c7638153f9b26a60e7111f71a0ff63d9571704905d3ca6df0b031747").unwrap(),
            signature: HexBinary::from_hex("82f5d3d2de4db19d40a6980e8aa37842a0e55d1df06bd68bddc8d60002e8e959eb9cfa368b3c1b77d18f02a54fe047b80f0989315f83b12a74fd8679c4f12aae86eaf6ab5690b34f1fddd50ee3cc6f6cdf59e95526d5a5d82aaa84fa6f181e42").unwrap(),
        };

        // Execute 1
        let info = mock_info("anyone", &[]);
        let response = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();
        let randomness_attr = response
            .attributes
            .iter()
            .find(|Attribute { key, .. }| key == "randomness")
            .unwrap();
        assert_eq!(
            randomness_attr.value,
            "8b676484b5fb1f37f9ec5c413d7d29883504e5b669f604a1ce68b3388e9ae3d9"
        );

        // Execute 2
        let info = mock_info("someone else", &[]);
        let response = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let randomness_attr = response
            .attributes
            .iter()
            .find(|Attribute { key, .. }| key == "randomness")
            .unwrap();
        assert_eq!(
            randomness_attr.value,
            "8b676484b5fb1f37f9ec5c413d7d29883504e5b669f604a1ce68b3388e9ae3d9"
        );
    }

    #[test]
    fn add_round_fails_when_same_bot_submits_multiple_times() {
        let mut deps = mock_dependencies();

        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg { test_mode: true };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg = ExecuteMsg::AddRound {
            // curl -sS https://drand.cloudflare.com/public/72785
            round: 72785,
            previous_signature: HexBinary::from_hex("a609e19a03c2fcc559e8dae14900aaefe517cb55c840f6e69bc8e4f66c8d18e8a609685d9917efbfb0c37f058c2de88f13d297c7e19e0ab24813079efe57a182554ff054c7638153f9b26a60e7111f71a0ff63d9571704905d3ca6df0b031747").unwrap(),
            signature: HexBinary::from_hex("82f5d3d2de4db19d40a6980e8aa37842a0e55d1df06bd68bddc8d60002e8e959eb9cfa368b3c1b77d18f02a54fe047b80f0989315f83b12a74fd8679c4f12aae86eaf6ab5690b34f1fddd50ee3cc6f6cdf59e95526d5a5d82aaa84fa6f181e42").unwrap(),
        };

        // Execute A1
        let info = mock_info("bot_alice", &[]);
        execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();
        // Execute B1
        let info = mock_info("bot_bob", &[]);
        execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();

        // Execute A2
        let info = mock_info("bot_alice", &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
        assert!(matches!(err, ContractError::SubmissionExists));
        // Execute B2
        let info = mock_info("bot_alice", &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert!(matches!(err, ContractError::SubmissionExists));
    }

    #[test]
    fn query_beacons_asc_works() {
        let mut deps = mock_dependencies();

        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg { test_mode: true };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        add_test_rounds(deps.as_mut(), "anyone");

        // Unlimited
        let BeaconsResponse { beacons } = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::BeaconsAsc {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();
        let response_rounds = beacons.iter().map(|b| b.round).collect::<Vec<u64>>();
        assert_eq!(response_rounds, [72785, 72786, 72787]);

        // Limit 2
        let BeaconsResponse { beacons } = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::BeaconsAsc {
                    start_after: None,
                    limit: Some(2),
                },
            )
            .unwrap(),
        )
        .unwrap();
        let response_rounds = beacons.iter().map(|b| b.round).collect::<Vec<u64>>();
        assert_eq!(response_rounds, [72785, 72786]);

        // After 0
        let BeaconsResponse { beacons } = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::BeaconsAsc {
                    start_after: Some(0),
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();
        let response_rounds = beacons.iter().map(|b| b.round).collect::<Vec<u64>>();
        assert_eq!(response_rounds, [72785, 72786, 72787]);

        // After 72785
        let BeaconsResponse { beacons } = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::BeaconsAsc {
                    start_after: Some(72785),
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();
        let response_rounds = beacons.iter().map(|b| b.round).collect::<Vec<u64>>();
        assert_eq!(response_rounds, [72786, 72787]);

        // After 72787
        let BeaconsResponse { beacons } = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::BeaconsAsc {
                    start_after: Some(72787),
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();
        let response_rounds = beacons.iter().map(|b| b.round).collect::<Vec<u64>>();
        assert_eq!(response_rounds, Vec::<u64>::new());
    }

    #[test]
    fn query_beacons_desc_works() {
        let mut deps = mock_dependencies();

        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg { test_mode: true };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        add_test_rounds(deps.as_mut(), "anyone");

        // Unlimited
        let BeaconsResponse { beacons } = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::BeaconsDesc {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();
        let response_rounds = beacons.iter().map(|b| b.round).collect::<Vec<u64>>();
        assert_eq!(response_rounds, [72787, 72786, 72785]);

        // Limit 2
        let BeaconsResponse { beacons } = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::BeaconsDesc {
                    start_after: None,
                    limit: Some(2),
                },
            )
            .unwrap(),
        )
        .unwrap();
        let response_rounds = beacons.iter().map(|b| b.round).collect::<Vec<u64>>();
        assert_eq!(response_rounds, [72787, 72786]);

        // After 99999
        let BeaconsResponse { beacons } = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::BeaconsDesc {
                    start_after: Some(99999),
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();
        let response_rounds = beacons.iter().map(|b| b.round).collect::<Vec<u64>>();
        assert_eq!(response_rounds, [72787, 72786, 72785]);

        // After 72787
        let BeaconsResponse { beacons } = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::BeaconsDesc {
                    start_after: Some(72787),
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();
        let response_rounds = beacons.iter().map(|b| b.round).collect::<Vec<u64>>();
        assert_eq!(response_rounds, [72786, 72785]);

        // After 72785
        let BeaconsResponse { beacons } = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::BeaconsDesc {
                    start_after: Some(72785),
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();
        let response_rounds = beacons.iter().map(|b| b.round).collect::<Vec<u64>>();
        assert_eq!(response_rounds, Vec::<u64>::new());
    }

    #[test]
    fn query_submissions_works() {
        let mut deps = mock_dependencies();

        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg { test_mode: true };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        add_test_rounds(deps.as_mut(), "bot1");

        // No submissions
        let response: SubmissionsResponse = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Submissions { round: 72777 },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(response.round, 72777);
        assert_eq!(response.submissions, Vec::<_>::new());

        // One submission
        let response: SubmissionsResponse = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Submissions { round: 72785 },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(response.round, 72785);
        assert_eq!(
            response.submissions,
            [Submission {
                bot: Addr::unchecked("bot1"),
                time: Timestamp::from_nanos(1571797419879305533),
            }]
        );

        add_test_rounds(deps.as_mut(), "bot2");

        // Two submissions
        let response: SubmissionsResponse = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Submissions { round: 72785 },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(response.round, 72785);
        assert_eq!(
            response.submissions,
            [
                Submission {
                    bot: Addr::unchecked("bot1"),
                    time: Timestamp::from_nanos(1571797419879305533),
                },
                Submission {
                    bot: Addr::unchecked("bot2"),
                    time: Timestamp::from_nanos(1571797419879305533),
                }
            ]
        );
    }

    /// Adds round 72785, 72786, 72787
    fn add_test_rounds(mut deps: DepsMut, bot_addr: &str) {
        let msg = ExecuteMsg::AddRound {
            // curl -sS https://drand.cloudflare.com/public/72785
            round: 72785,
            previous_signature: HexBinary::from_hex("a609e19a03c2fcc559e8dae14900aaefe517cb55c840f6e69bc8e4f66c8d18e8a609685d9917efbfb0c37f058c2de88f13d297c7e19e0ab24813079efe57a182554ff054c7638153f9b26a60e7111f71a0ff63d9571704905d3ca6df0b031747").unwrap(),
            signature: HexBinary::from_hex("82f5d3d2de4db19d40a6980e8aa37842a0e55d1df06bd68bddc8d60002e8e959eb9cfa368b3c1b77d18f02a54fe047b80f0989315f83b12a74fd8679c4f12aae86eaf6ab5690b34f1fddd50ee3cc6f6cdf59e95526d5a5d82aaa84fa6f181e42").unwrap(),
        };
        let info = mock_info(bot_addr, &[]);
        execute(deps.branch(), mock_env(), info, msg).unwrap();
        let msg = ExecuteMsg::AddRound {
            // curl -sS https://drand.cloudflare.com/public/72786
            round: 72786,
            previous_signature: HexBinary::from_hex("82f5d3d2de4db19d40a6980e8aa37842a0e55d1df06bd68bddc8d60002e8e959eb9cfa368b3c1b77d18f02a54fe047b80f0989315f83b12a74fd8679c4f12aae86eaf6ab5690b34f1fddd50ee3cc6f6cdf59e95526d5a5d82aaa84fa6f181e42").unwrap(),
            signature: HexBinary::from_hex("85d64193239c6a2805b5953521c1e7c412d13f8b29df2dfc796b7dc8e1fd795b764362e49302956a350f9385f68b68d8085fda08c2bd0528984a413db52860b408c72d1210609de3a342259d4c08f86ee729a2dbeb140908270849fd7d0dec40").unwrap(),
        };
        let info = mock_info(bot_addr, &[]);
        execute(deps.branch(), mock_env(), info, msg).unwrap();
        let msg = ExecuteMsg::AddRound {
            // curl -sS https://drand.cloudflare.com/public/72787
            round: 72787,
            previous_signature: HexBinary::from_hex("85d64193239c6a2805b5953521c1e7c412d13f8b29df2dfc796b7dc8e1fd795b764362e49302956a350f9385f68b68d8085fda08c2bd0528984a413db52860b408c72d1210609de3a342259d4c08f86ee729a2dbeb140908270849fd7d0dec40").unwrap(),
            signature: HexBinary::from_hex("8ceee95d523f54a752807f4705ce0f89e69911dd3dce330a337b9409905a881a2f879d48fce499bfeeb3b12e7f83ab7d09b42f31fa729af4c19adfe150075b2f3fe99c8fbcd7b0b5f0bb91ac8ad8715bfe52e3fb12314fddb76d4e42461f6ea4").unwrap(),
        };
        let info = mock_info(bot_addr, &[]);
        execute(deps.branch(), mock_env(), info, msg).unwrap();
    }

    //
    // IBC tests
    //

    #[test]
    fn enforce_version_in_handshake() {
        let mut deps = setup();

        let wrong_order = mock_ibc_channel_open_try("channel-12", BAD_APP_ORDER, IBC_APP_VERSION);
        ibc_channel_open(deps.as_mut(), mock_env(), wrong_order).unwrap_err();

        let wrong_version = mock_ibc_channel_open_try("channel-12", APP_ORDER, "another version");
        ibc_channel_open(deps.as_mut(), mock_env(), wrong_version).unwrap_err();

        let valid_handshake = mock_ibc_channel_open_try("channel-12", APP_ORDER, IBC_APP_VERSION);
        ibc_channel_open(deps.as_mut(), mock_env(), valid_handshake).unwrap();
    }

    #[test]
    fn proper_handshake_flow() {
        let mut deps = setup();
        let channel_id = "channel-1234";

        // first we try to open with a valid handshake
        let handshake_open = mock_ibc_channel_open_init(channel_id, APP_ORDER, IBC_APP_VERSION);
        ibc_channel_open(deps.as_mut(), mock_env(), handshake_open).unwrap();

        // then we connect (with counter-party version set)
        let handshake_connect =
            mock_ibc_channel_connect_ack(channel_id, APP_ORDER, IBC_APP_VERSION);
        let _res = ibc_channel_connect(deps.as_mut(), mock_env(), handshake_connect).unwrap();
    }

    #[test]
    fn check_close_channel() {
        let mut deps = setup();

        let channel_id = "channel-123";
        let account = "acct-123";

        // register the channel
        connect(deps.as_mut(), channel_id, account);
        // assign it some funds
        let funds = vec![coin(123456, "uatom"), coin(7654321, "tgrd")];
        deps.querier.update_balance(account, funds);

        // close the channel
        let channel = mock_ibc_channel_close_init(channel_id, APP_ORDER, IBC_APP_VERSION);
        let _res = ibc_channel_close(deps.as_mut(), mock_env(), channel).unwrap();
    }

    //
    // Other
    //

    #[test]
    fn next_round_works_for_test_mode() {
        let mut deps = mock_dependencies();
        let (round, source_id) = next_round(&mut deps.storage, NextRoundMode::Test).unwrap();
        assert_eq!(round, 2183660);
        assert_eq!(source_id, "test-mode:2183660");
        let (round, source_id) = next_round(&mut deps.storage, NextRoundMode::Test).unwrap();
        assert_eq!(round, 2183661);
        assert_eq!(source_id, "test-mode:2183661");
        let (round, source_id) = next_round(&mut deps.storage, NextRoundMode::Test).unwrap();
        assert_eq!(round, 2183662);
        assert_eq!(source_id, "test-mode:2183662");
    }

    #[test]
    fn next_round_works_for_time_mode() {
        let mut deps = mock_dependencies();

        // UNIX epoch
        let (round, _) = next_round(
            &mut deps.storage,
            NextRoundMode::Time {
                base: Timestamp::from_seconds(0),
            },
        )
        .unwrap();
        assert_eq!(round, 1);

        // Before Drand genesis (https://api3.drand.sh/info)
        let (round, _) = next_round(
            &mut deps.storage,
            NextRoundMode::Time {
                base: Timestamp::from_seconds(1595431050).minus_nanos(1),
            },
        )
        .unwrap();
        assert_eq!(round, 1);

        // At Drand genesis (https://api3.drand.sh/info)
        let (round, _) = next_round(
            &mut deps.storage,
            NextRoundMode::Time {
                base: Timestamp::from_seconds(1595431050),
            },
        )
        .unwrap();
        assert_eq!(round, 2);

        // After Drand genesis (https://api3.drand.sh/info)
        let (round, _) = next_round(
            &mut deps.storage,
            NextRoundMode::Time {
                base: Timestamp::from_seconds(1595431050).plus_nanos(1),
            },
        )
        .unwrap();
        assert_eq!(round, 2);

        // Drand genesis +29s/30s/31s
        let (round, _) = next_round(
            &mut deps.storage,
            NextRoundMode::Time {
                base: Timestamp::from_seconds(1595431050).plus_seconds(29),
            },
        )
        .unwrap();
        assert_eq!(round, 2);
        let (round, _) = next_round(
            &mut deps.storage,
            NextRoundMode::Time {
                base: Timestamp::from_seconds(1595431050).plus_seconds(30),
            },
        )
        .unwrap();
        assert_eq!(round, 3);
        let (round, _) = next_round(
            &mut deps.storage,
            NextRoundMode::Time {
                base: Timestamp::from_seconds(1595431050).plus_seconds(31),
            },
        )
        .unwrap();
        assert_eq!(round, 3);
    }
}
