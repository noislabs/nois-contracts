use cosmwasm_std::{
    ensure_eq, entry_point, from_binary, from_slice, to_binary, Attribute, Deps, DepsMut, Empty,
    Env, Event, HexBinary, Ibc3ChannelOpenResponse, IbcBasicResponse, IbcChannelCloseMsg,
    IbcChannelConnectMsg, IbcChannelOpenMsg, IbcChannelOpenResponse, IbcPacketAckMsg,
    IbcPacketReceiveMsg, IbcPacketTimeoutMsg, IbcReceiveResponse, MessageInfo, QueryResponse,
    Response, StdResult,
};
use nois_protocol::{
    check_order, check_version, DeliverBeaconPacketAck, Never, RequestBeaconPacket, StdAck,
    IBC_APP_VERSION,
};

use crate::error::ContractError;
use crate::job_id::validate_job_id;
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, JobStatsResponse, QueryMsg};
use crate::request_router::{NewDrand, RequestRouter, RoutingReceipt};
use crate::state::{get_processed_jobs, unprocessed_jobs_len, Config, CONFIG};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let config = Config {
        drand_contract: None,
        min_round: msg.min_round,
        incentive_amount: msg.incentive_amount,
        incentive_denom: msg.incentive_denom,
    };
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::default())
}

// This no-op migrate implementation allows us to upgrade within the 0.7 series.
// No state changes expected.
#[entry_point]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: Empty) -> StdResult<Response> {
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
        ExecuteMsg::AddVerifiedRound { round, randomness } => {
            execute_add_verified_round(deps, env, info, round, randomness)
        }
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<QueryResponse> {
    let response = match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?)?,
        QueryMsg::JobStats { round } => to_binary(&query_job_stats(deps, round)?)?,
    };
    Ok(response)
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config)
}

// Query job stats by round
fn query_job_stats(deps: Deps, round: u64) -> StdResult<JobStatsResponse> {
    let unprocessed = unprocessed_jobs_len(deps.storage, round)?;
    let processed = get_processed_jobs(deps.storage, round)?;
    Ok(JobStatsResponse {
        round,
        unprocessed,
        processed,
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
) -> Result<IbcReceiveResponse, Never> {
    let packet = msg.packet;
    // which local channel did this packet come on
    let channel = packet.dest.channel_id;

    // put this in a closure so we can convert all error responses into acknowledgements
    (|| {
        let msg: RequestBeaconPacket = from_slice(&packet.data)?;
        receive_request_beacon(deps, env, channel, msg)
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
    deps: DepsMut,
    env: Env,
    channel: String,
    msg: RequestBeaconPacket,
) -> Result<IbcReceiveResponse, ContractError> {
    let RequestBeaconPacket {
        sender,
        after,
        job_id,
    } = msg;
    validate_job_id(&job_id)?;

    let router = RequestRouter::new();
    let RoutingReceipt {
        acknowledgement,
        msgs,
    } = router.route(deps, env, channel, after, sender, job_id)?;

    Ok(IbcReceiveResponse::new()
        .set_ack(acknowledgement)
        .add_messages(msgs)
        .add_attribute("action", "receive_request_beacon"))
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

/// This method simulates how the drand contract will call the front-desk contract to inform
/// it when there are is a new round. Here the verification was done at a trusted source so
/// we only send the raw randomness.
fn execute_add_verified_round(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    round: u64,
    randomness: HexBinary,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if let Some(drand_contract) = config.drand_contract {
        ensure_eq!(
            info.sender,
            drand_contract,
            ContractError::UnauthorizedAddVerifiedRound
        );
    }

    let mut attributes = Vec::<Attribute>::new();
    let router = RequestRouter::new();
    let NewDrand {
        msgs,
        jobs_processed,
        jobs_left,
    } = router.new_drand(deps, env, round, &randomness)?;
    attributes.push(Attribute::new("jobs_processed", jobs_processed.to_string()));
    attributes.push(Attribute::new("jobs_left", jobs_left.to_string()));

    Ok(Response::new()
        .add_messages(msgs)
        .add_attributes(attributes))
}

#[cfg(test)]
mod tests {

    use crate::msg::ExecuteMsg;

    use super::*;
    use cosmwasm_std::testing::{
        mock_dependencies, mock_env, mock_ibc_channel_close_init, mock_ibc_channel_connect_ack,
        mock_ibc_channel_open_init, mock_ibc_channel_open_try, mock_ibc_packet_recv, mock_info,
        MockApi, MockQuerier, MockStorage,
    };
    use cosmwasm_std::{coin, from_binary, CosmosMsg, IbcMsg, OwnedDeps, Timestamp, Uint128};
    use nois_protocol::{APP_ORDER, BAD_APP_ORDER};

    const CREATOR: &str = "creator";
    const TESTING_MIN_ROUND: u64 = 72785;

    fn setup() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            min_round: TESTING_MIN_ROUND,
            incentive_amount: Uint128::new(1_000_000),
            incentive_denom: "unois".to_string(),
        };
        let info = mock_info(CREATOR, &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        deps
    }

    fn make_add_verified_round_msg(round: u64) -> ExecuteMsg {
        match round {
            9 => ExecuteMsg::AddVerifiedRound {
                // curl -sS https://drand.cloudflare.com/public/9
                round: 9,
                randomness: HexBinary::from_hex(
                    "1b9acda1c43e333bcf02ddce634b18ff79803a904097a5896710c7ae798b47ab",
                )
                .unwrap(),
            },
            72785 => ExecuteMsg::AddVerifiedRound {
                // curl -sS https://drand.cloudflare.com/public/72785
                round: 72785,
                randomness: HexBinary::from_hex(
                    "8b676484b5fb1f37f9ec5c413d7d29883504e5b669f604a1ce68b3388e9ae3d9",
                )
                .unwrap(),
            },
            72786 => ExecuteMsg::AddVerifiedRound {
                // curl -sS https://drand.cloudflare.com/public/72786
                round: 72786,
                randomness: HexBinary::from_hex(
                    "0ed47e6ebc311192000df4469bb5a5a00445a9365e428d61c8c08d78dd1e51a8",
                )
                .unwrap(),
            },
            72787 => ExecuteMsg::AddVerifiedRound {
                // curl -sS https://drand.cloudflare.com/public/72787
                round: 72787,
                randomness: HexBinary::from_hex(
                    "d4ea3e5e43bf510c1b086613a9e68257b317202dbe5aab1b9182b65f51f4b82c",
                )
                .unwrap(),
            },
            2183668 => ExecuteMsg::AddVerifiedRound {
                // curl -sS https://drand.cloudflare.com/public/2183668
                round: 2183668,
                randomness: HexBinary::from_hex(
                    "3436462283a07e695c41854bb953e5964d8737e7e29745afe54a9f4897b6c319",
                )
                .unwrap(),
            },
            2183669 => ExecuteMsg::AddVerifiedRound {
                // curl -sS https://drand.cloudflare.com/public/2183669
                round: 2183669,
                randomness: HexBinary::from_hex(
                    "408de94b8c7e1972b06a4ab7636eb1ba2a176022a30d018c3b55e89289d41149",
                )
                .unwrap(),
            },
            2183670 => ExecuteMsg::AddVerifiedRound {
                // curl -sS https://drand.cloudflare.com/public/2183670
                round: 2183670,
                randomness: HexBinary::from_hex(
                    "e5f7ba655389eee248575dde70cb9f3293c9774c8538136a135601907158d957",
                )
                .unwrap(),
            },
            2183671 => ExecuteMsg::AddVerifiedRound {
                // curl -sS https://drand.cloudflare.com/public/2183671
                round: 2183671,
                randomness: HexBinary::from_hex(
                    "324e2a196293b42806c12c7bbd1aeba8d5617942f152a16588223f905f60801a",
                )
                .unwrap(),
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

        let msg = InstantiateMsg {
            min_round: TESTING_MIN_ROUND,
            incentive_amount: Uint128::new(1_000_000),
            incentive_denom: "unois".to_string(),
        };
        let info = mock_info("creator", &[]);
        let env = mock_env();
        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let config: ConfigResponse =
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
        assert_eq!(
            config,
            ConfigResponse {
                drand_contract: None,
                min_round: TESTING_MIN_ROUND,
                incentive_amount: Uint128::new(1_000_000),
                incentive_denom: "unois".to_string(),
            }
        );
    }

    //
    // Execute tests
    //

    #[test]
    fn add_round_verified_processes_jobs() {
        let mut deps = mock_dependencies();

        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            min_round: TESTING_MIN_ROUND,
            incentive_amount: Uint128::new(1_000_000),
            incentive_denom: "unois".to_string(),
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Create one job
        let msg = mock_ibc_packet_recv(
            "foo",
            &RequestBeaconPacket {
                after: Timestamp::from_seconds(1660941090 - 1),
                job_id: "test 1".to_string(),
                sender: "my_dapp".to_string(),
            },
        )
        .unwrap();
        ibc_packet_receive(deps.as_mut(), mock_env(), msg).unwrap();

        // Previous round processes no job
        let msg = make_add_verified_round_msg(2183668);
        let res = execute(deps.as_mut(), mock_env(), mock_info("anon", &[]), msg).unwrap();
        assert_eq!(res.messages.len(), 0);
        let jobs_processed = first_attr(&res.attributes, "jobs_processed").unwrap();
        assert_eq!(jobs_processed, "0");
        let jobs_left = first_attr(&res.attributes, "jobs_left").unwrap();
        assert_eq!(jobs_left, "0");

        // Process one job
        let msg = make_add_verified_round_msg(2183669);
        let res = execute(deps.as_mut(), mock_env(), mock_info("anon", &[]), msg).unwrap();
        assert_eq!(res.messages.len(), 1);
        assert_eq!(res.messages[0].gas_limit, None);
        assert!(matches!(
            res.messages[0].msg,
            CosmosMsg::Ibc(IbcMsg::SendPacket { .. })
        ));
        let jobs_processed = first_attr(&res.attributes, "jobs_processed").unwrap();
        assert_eq!(jobs_processed, "1");
        let jobs_left = first_attr(&res.attributes, "jobs_left").unwrap();
        assert_eq!(jobs_left, "0");

        // Create 3 job
        for i in 0..3 {
            let msg = mock_ibc_packet_recv(
                "foo",
                &RequestBeaconPacket {
                    after: Timestamp::from_seconds(1660941120 - 1),
                    job_id: format!("test {i}"),
                    sender: "my_dapp".to_string(),
                },
            )
            .unwrap();
            ibc_packet_receive(deps.as_mut(), mock_env(), msg).unwrap();
        }

        // Process 3 jobs
        let msg = make_add_verified_round_msg(2183670);
        let res = execute(deps.as_mut(), mock_env(), mock_info("anon", &[]), msg).unwrap();
        assert_eq!(res.messages.len(), 3);
        assert_eq!(res.messages[0].gas_limit, None);
        assert_eq!(res.messages[1].gas_limit, None);
        assert_eq!(res.messages[2].gas_limit, None);
        assert!(matches!(
            res.messages[0].msg,
            CosmosMsg::Ibc(IbcMsg::SendPacket { .. })
        ));
        assert!(matches!(
            res.messages[1].msg,
            CosmosMsg::Ibc(IbcMsg::SendPacket { .. })
        ));
        assert!(matches!(
            res.messages[2].msg,
            CosmosMsg::Ibc(IbcMsg::SendPacket { .. })
        ));
        let jobs_processed = first_attr(&res.attributes, "jobs_processed").unwrap();
        assert_eq!(jobs_processed, "3");
        let jobs_left = first_attr(&res.attributes, "jobs_left").unwrap();
        assert_eq!(jobs_left, "0");

        // Create 7 job
        for i in 0..7 {
            let msg = mock_ibc_packet_recv(
                "foo",
                &RequestBeaconPacket {
                    after: Timestamp::from_seconds(1660941150 - 1),
                    job_id: format!("test {i}"),
                    sender: "my_dapp".to_string(),
                },
            )
            .unwrap();
            ibc_packet_receive(deps.as_mut(), mock_env(), msg).unwrap();
        }

        // Process first 3 jobs
        let msg = make_add_verified_round_msg(2183671);
        let res = execute(deps.as_mut(), mock_env(), mock_info("anon1", &[]), msg).unwrap();
        assert_eq!(res.messages.len(), 3);
        let jobs_processed = first_attr(&res.attributes, "jobs_processed").unwrap();
        assert_eq!(jobs_processed, "3");
        let jobs_left = first_attr(&res.attributes, "jobs_left").unwrap();
        assert_eq!(jobs_left, "4");

        // Process next 3 jobs
        let msg = make_add_verified_round_msg(2183671);
        let res = execute(deps.as_mut(), mock_env(), mock_info("anon2", &[]), msg).unwrap();
        assert_eq!(res.messages.len(), 3);
        let jobs_processed = first_attr(&res.attributes, "jobs_processed").unwrap();
        assert_eq!(jobs_processed, "3");
        let jobs_left = first_attr(&res.attributes, "jobs_left").unwrap();
        assert_eq!(jobs_left, "1");

        // Process last 1 jobs
        let msg = make_add_verified_round_msg(2183671);
        let res = execute(deps.as_mut(), mock_env(), mock_info("anon3", &[]), msg).unwrap();
        assert_eq!(res.messages.len(), 1);
        let jobs_processed = first_attr(&res.attributes, "jobs_processed").unwrap();
        assert_eq!(jobs_processed, "1");
        let jobs_left = first_attr(&res.attributes, "jobs_left").unwrap();
        assert_eq!(jobs_left, "0");

        // No jobs left for later submissions
        let msg = make_add_verified_round_msg(2183671);
        let res = execute(deps.as_mut(), mock_env(), mock_info("anon4", &[]), msg).unwrap();
        assert_eq!(res.messages.len(), 0);
        let jobs_processed = first_attr(&res.attributes, "jobs_processed").unwrap();
        assert_eq!(jobs_processed, "0");
        let jobs_left = first_attr(&res.attributes, "jobs_left").unwrap();
        assert_eq!(jobs_left, "0");
    }

    //
    // Query tests
    //

    #[test]
    fn query_job_stats_works() {
        let mut deps = mock_dependencies();

        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            min_round: TESTING_MIN_ROUND,
            incentive_amount: Uint128::new(1_000_000),
            incentive_denom: "unois".to_string(),
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        fn job_stats(deps: Deps, round: u64) -> JobStatsResponse {
            from_binary(&query(deps, mock_env(), QueryMsg::JobStats { round }).unwrap()).unwrap()
        }

        // No jobs by default
        assert_eq!(
            job_stats(deps.as_ref(), 2183669),
            JobStatsResponse {
                round: 2183669,
                processed: 0,
                unprocessed: 0,
            }
        );

        // Create one job
        let msg = mock_ibc_packet_recv(
            "foo",
            &RequestBeaconPacket {
                after: Timestamp::from_seconds(1660941090 - 1),
                job_id: "test 1".to_string(),
                sender: "my_dapp".to_string(),
            },
        )
        .unwrap();
        ibc_packet_receive(deps.as_mut(), mock_env(), msg).unwrap();

        // One unprocessed job
        assert_eq!(
            job_stats(deps.as_ref(), 2183669),
            JobStatsResponse {
                round: 2183669,
                processed: 0,
                unprocessed: 1,
            }
        );

        let msg = make_add_verified_round_msg(2183669);
        execute(deps.as_mut(), mock_env(), mock_info("bot", &[]), msg).unwrap();

        // 1 processed job, no unprocessed jobs
        assert_eq!(
            job_stats(deps.as_ref(), 2183669),
            JobStatsResponse {
                round: 2183669,
                processed: 1,
                unprocessed: 0,
            }
        );

        // New job for existing round gets processed immediately
        let msg = mock_ibc_packet_recv(
            "foo",
            &RequestBeaconPacket {
                after: Timestamp::from_seconds(1660941090 - 1),
                job_id: "test 2".to_string(),
                sender: "my_dapp".to_string(),
            },
        )
        .unwrap();
        ibc_packet_receive(deps.as_mut(), mock_env(), msg).unwrap();

        // 2 processed job, no unprocessed jobs
        // assert_eq!(
        //     job_stats(deps.as_ref(), 2183669),
        //     JobStatsResponse {
        //         round: 2183669,
        //         processed: 2,
        //         unprocessed: 0,
        //     }
        // );

        // Create 20 jobs
        for i in 0..20 {
            let msg = mock_ibc_packet_recv(
                "foo",
                &RequestBeaconPacket {
                    after: Timestamp::from_seconds(1660941150 - 1),
                    job_id: format!("job {i}"),
                    sender: "my_dapp".to_string(),
                },
            )
            .unwrap();
            ibc_packet_receive(deps.as_mut(), mock_env(), msg).unwrap();
        }

        // 20 unprocessed
        assert_eq!(
            job_stats(deps.as_ref(), 2183671),
            JobStatsResponse {
                round: 2183671,
                processed: 0,
                unprocessed: 20,
            }
        );

        // process some
        let msg = make_add_verified_round_msg(2183671);
        execute(deps.as_mut(), mock_env(), mock_info("bot", &[]), msg).unwrap();

        // Some processed, rest unprocessed
        assert_eq!(
            job_stats(deps.as_ref(), 2183671),
            JobStatsResponse {
                round: 2183671,
                processed: 3,
                unprocessed: 17,
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
}
