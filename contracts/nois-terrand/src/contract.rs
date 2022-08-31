use cosmwasm_std::{
    entry_point, from_binary, from_slice, to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, Event,
    Ibc3ChannelOpenResponse, IbcBasicResponse, IbcChannelCloseMsg, IbcChannelConnectMsg,
    IbcChannelOpenMsg, IbcChannelOpenResponse, IbcMsg, IbcPacketAckMsg, IbcPacketReceiveMsg,
    IbcPacketTimeoutMsg, IbcReceiveResponse, MessageInfo, Order, QueryResponse, Response, StdError,
    StdResult, Storage, Timestamp,
};
use drand_verify::{derive_randomness, g1_from_variable, verify};
use nois_ibc_protocol::{
    check_order, check_version, Beacon, DeliverBeaconPacket, DeliverBeaconPacketAck,
    RequestBeaconPacket, RequestBeaconPacketAck, StdAck, IBC_APP_VERSION,
};

use crate::error::ContractError;
use crate::msg::{
    BeaconReponse, ConfigResponse, ExecuteMsg, InstantiateMsg, LatestRandomResponse, QueryMsg,
};
use crate::state::{Config, Job, BEACONS, CONFIG, DRAND_JOBS, TEST_MODE_NEXT_ROUND};

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
        drand_pubkey: msg.pubkey,
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
        QueryMsg::LatestDrand {} => to_binary(&query_latest(deps)?)?,
    };
    Ok(response)
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config)
}

// Query beacon by round
fn query_beacon(deps: Deps, round: u64) -> StdResult<BeaconReponse> {
    let beacon = BEACONS.may_load(deps.storage, round)?;
    Ok(BeaconReponse { beacon })
}

// Query latest beacon
fn query_latest(deps: Deps) -> StdResult<LatestRandomResponse> {
    let mut iter = BEACONS.range(deps.storage, None, None, Order::Descending);
    let (key, value) = iter
        .next()
        .ok_or_else(|| StdError::generic_err("Not found"))??;

    Ok(LatestRandomResponse {
        round: key,
        randomness: value.randomness,
    })
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
    receive_get_beacon(deps, env, channel, msg.after, msg.sender, msg.callback_id)
}

fn receive_get_beacon(
    deps: DepsMut,
    env: Env,
    channel: String,
    after: Timestamp,
    sender: String,
    callback_id: Option<String>,
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
        callback_id,
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

fn process_job(blocktime: Timestamp, job: Job, beacon: &Beacon) -> Result<IbcMsg, ContractError> {
    let packet = DeliverBeaconPacket {
        sender: job.sender,
        callback_id: job.callback_id,
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

/// See https://drand.love/developer/
const DRAND_CHAIN_HASH: &str = "8990e7a9aaed2ffed73dbd7092123d6f289930540d7651336225dc172e51b2ce";
const DRAND_GENESIS: Timestamp = Timestamp::from_seconds(1595431050);
const DRAND_ROUND_LENGTH: u64 = 30_000_000_000; // in nanoseconds

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
                let next_round = (from_genesis / DRAND_ROUND_LENGTH) + 1;
                next_round + 1
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
    previous_signature: Binary,
    signature: Binary,
) -> Result<Response, ContractError> {
    // Handle sender is not sending funds
    if !info.funds.is_empty() {
        return Err(StdError::generic_err("Do not send funds").into());
    }

    // Sender is not adding new rounds.
    // Unclear if this is supposed to be an error (i.e. fail/revert the whole transaction)
    // but let's see.
    if BEACONS.has(deps.storage, round) {
        return Err(StdError::generic_err(format!("Round already {} added", round)).into());
    };

    let config = CONFIG.load(deps.storage)?;

    let pk = g1_from_variable(&config.drand_pubkey).map_err(|_| ContractError::InvalidPubkey {})?;
    let is_valid = verify(&pk, round, &previous_signature, &signature).unwrap_or(false);

    if !is_valid {
        return Err(ContractError::InvalidSignature {});
    }

    let randomness = derive_randomness(signature.as_slice());
    let randomness_hex = hex::encode(&randomness);

    let beacon = &Beacon {
        randomness: randomness_hex.clone(),
    };
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
        .add_attribute("round", round.to_string())
        .add_attribute("randomness", randomness_hex)
        .add_attribute("worker", info.sender.to_string()))
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
    use nois_ibc_protocol::{APP_ORDER, BAD_APP_ORDER};

    const CREATOR: &str = "creator";

    fn setup() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            pubkey: pubkey_loe_mainnet(),
            test_mode: true,
        };
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

    // $ node
    // > Uint8Array.from(Buffer.from("868f005eb8e6e4ca0a47c8a77ceaa5309a47978a7c71bc5cce96366b5d7a569937c529eeda66c7293784a9402801af31", "hex"))
    fn pubkey_loe_mainnet() -> Binary {
        vec![
            134, 143, 0, 94, 184, 230, 228, 202, 10, 71, 200, 167, 124, 234, 165, 48, 154, 71, 151,
            138, 124, 113, 188, 92, 206, 150, 54, 107, 93, 122, 86, 153, 55, 197, 41, 238, 218,
            102, 199, 41, 55, 132, 169, 64, 40, 1, 175, 49,
        ]
        .into()
    }

    //
    // Instantiate tests
    //

    #[test]
    fn instantiate_works() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            pubkey: pubkey_loe_mainnet(),
            test_mode: true,
        };
        let info = mock_info("creator", &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len())
    }

    //
    // Execute tests
    //

    #[test]
    fn add_round_verifies_and_stores_randomness() {
        let mut deps = mock_dependencies();

        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            pubkey: pubkey_loe_mainnet(),
            test_mode: true,
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("anyone", &[]);
        let msg = ExecuteMsg::AddRound {
            // curl -sS https://drand.cloudflare.com/public/72785
            round: 72785,
            previous_signature: hex::decode("a609e19a03c2fcc559e8dae14900aaefe517cb55c840f6e69bc8e4f66c8d18e8a609685d9917efbfb0c37f058c2de88f13d297c7e19e0ab24813079efe57a182554ff054c7638153f9b26a60e7111f71a0ff63d9571704905d3ca6df0b031747").unwrap().into(),
            signature: hex::decode("82f5d3d2de4db19d40a6980e8aa37842a0e55d1df06bd68bddc8d60002e8e959eb9cfa368b3c1b77d18f02a54fe047b80f0989315f83b12a74fd8679c4f12aae86eaf6ab5690b34f1fddd50ee3cc6f6cdf59e95526d5a5d82aaa84fa6f181e42").unwrap().into(),
        };
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let response: BeaconReponse = from_binary(
            &query(deps.as_ref(), mock_env(), QueryMsg::Beacon { round: 72785 }).unwrap(),
        )
        .unwrap();
        assert_eq!(
            response.beacon.unwrap().randomness,
            "8b676484b5fb1f37f9ec5c413d7d29883504e5b669f604a1ce68b3388e9ae3d9"
        );
    }

    #[test]
    fn add_round_fails_when_pubkey_is_invalid() {
        let mut deps = mock_dependencies();

        let info = mock_info("creator", &[]);
        let mut broken: Vec<u8> = pubkey_loe_mainnet().into();
        broken.push(0xF9);
        let msg = InstantiateMsg {
            pubkey: broken.into(),
            test_mode: true,
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("anyone", &[]);
        let msg = ExecuteMsg::AddRound {
            // curl -sS https://drand.cloudflare.com/public/72785 | jq
            round: 72785,
            previous_signature: hex::decode("a609e19a03c2fcc559e8dae14900aaefe517cb55c840f6e69bc8e4f66c8d18e8a609685d9917efbfb0c37f058c2de88f13d297c7e19e0ab24813079efe57a182554ff054c7638153f9b26a60e7111f71a0ff63d9571704905d3ca6df0b031747").unwrap().into(),
            signature: hex::decode("82f5d3d2de4db19d40a6980e8aa37842a0e55d1df06bd68bddc8d60002e8e959eb9cfa368b3c1b77d18f02a54fe047b80f0989315f83b12a74fd8679c4f12aae86eaf6ab5690b34f1fddd50ee3cc6f6cdf59e95526d5a5d82aaa84fa6f181e42").unwrap().into(),
        };
        let result = execute(deps.as_mut(), mock_env(), info, msg);
        match result.unwrap_err() {
            ContractError::InvalidPubkey {} => {}
            err => panic!("Unexpected error: {:?}", err),
        }
    }

    #[test]
    fn add_round_fails_for_broken_signature() {
        let mut deps = mock_dependencies();

        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            pubkey: pubkey_loe_mainnet(),
            test_mode: true,
        };
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
        }
    }

    #[test]
    fn add_round_fails_for_invalid_signature() {
        let mut deps = mock_dependencies();

        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            pubkey: pubkey_loe_mainnet(),
            test_mode: true,
        };
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
        }
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
