use std::str::FromStr;

use cosmwasm_std::{
    ensure_eq, to_json_binary, Addr, Attribute, BankMsg, Coin, CosmosMsg, Deps, DepsMut, Empty,
    Env, HexBinary, MessageInfo, Order, QueryResponse, Response, StdError, StdResult, Uint128,
    WasmMsg,
};
use cw2::set_contract_version;
use cw_storage_plus::Bound;
use drand_common::DrandNetwork;
use drand_verify::{derive_randomness, G2PubkeyFastnet, G2PubkeyRfc, Pubkey};

use crate::attributes::{
    ATTR_BOT, ATTR_RANDOMNESS, ATTR_REWARD_PAYOUT, ATTR_REWARD_POINTS, ATTR_ROUND,
};
use crate::bots::{eligible_group, group, validate_moniker};
use crate::error::ContractError;
use crate::msg::{
    AllowlistResponse, BeaconResponse, BeaconsResponse, BotResponse, BotsResponse, ConfigResponse,
    ExecuteMsg, InstantiateMsg, IsAllowlistedResponse, IsIncentivizedResponse,
    NoisGatewayExecuteMsg, QueriedSubmission, QueryMsg, SubmissionsResponse,
};
use crate::state::{
    Bot, Config, QueriedBeacon, QueriedBot, StoredSubmission, VerifiedBeacon, ALLOWLIST, BEACONS,
    BOTS, CONFIG, FASTNET_SUBMISSIONS_COUNT, INCENTIVIZED_BY_GATEWAY,
    INCENTIVIZED_BY_GATEWAY_MARKER, QUICKNET_SUBMISSIONS_COUNT, SUBMISSIONS,
};

/// Constant defining how many submissions per round will be rewarded
const NUMBER_OF_INCENTIVES_PER_ROUND: u16 = 6;
const NUMBER_OF_SUBMISSION_VERIFICATION_PER_ROUND: u16 = 3;
/// Point system for rewarding submisisons.
///
/// We use small integers here which are later multiplied with a constant to
/// pay out the rewards.
/// For values up to 100 points per submission we can safely sum up `Number.MAX_SAFE_INTEGER / 100 = 90071992547409` times.
/// This is two submissions per minute for 85 million years or one submission per second for 3 million years.
const INCENTIVE_POINTS_FOR_VERIFICATION: u64 = 35;
const INCENTIVE_POINTS_FOR_FAST_BOT: u64 = 15;

#[cfg_attr(not(feature = "library"), ::cosmwasm_std::entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let manager = deps.api.addr_validate(&msg.manager)?;
    let config = Config {
        manager,
        gateway: None,
        min_round: msg.min_round,
        incentive_point_price: msg.incentive_point_price,
        incentive_denom: msg.incentive_denom,
    };
    CONFIG.save(deps.storage, &config)?;
    set_contract_version(
        deps.storage,
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
    )?;
    Ok(Response::default())
}

// This no-op migrate implementation allows us to upgrade within the 0.7 series.
// No state changes expected.
#[cfg_attr(not(feature = "library"), ::cosmwasm_std::entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: Empty) -> StdResult<Response> {
    set_contract_version(
        deps.storage,
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
    )?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), ::cosmwasm_std::entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AddRound {
            network,
            round,
            signature,
        } => execute_add_round(deps, env, info, network, round, signature),
        ExecuteMsg::RegisterBot { moniker } => execute_register_bot(deps, env, info, moniker),
        ExecuteMsg::SetIncentivized { round } => execute_set_incentivized(deps, env, info, round),
        ExecuteMsg::UpdateAllowlistBots { add, remove } => {
            execute_update_allowlist_bots(deps, info, add, remove)
        }
        ExecuteMsg::SetConfig {
            manager,
            gateway,
            min_round,
            incentive_point_price,
            incentive_denom,
        } => execute_set_config(
            deps,
            info,
            manager,
            gateway,
            min_round,
            incentive_point_price,
            incentive_denom,
        ),
    }
}

#[cfg_attr(not(feature = "library"), ::cosmwasm_std::entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<QueryResponse> {
    let response = match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?)?,
        QueryMsg::Beacon { round } => to_json_binary(&query_beacon(deps, round)?)?,
        QueryMsg::BeaconsAsc { start_after, limit } => {
            to_json_binary(&query_beacons(deps, start_after, limit, Order::Ascending)?)?
        }
        QueryMsg::BeaconsDesc { start_after, limit } => {
            to_json_binary(&query_beacons(deps, start_after, limit, Order::Descending)?)?
        }
        QueryMsg::IsIncentivized { sender, rounds } => {
            to_json_binary(&query_is_incentivized(deps, sender, rounds)?)?
        }
        QueryMsg::Submissions { round } => to_json_binary(&query_submissions(deps, round)?)?,
        QueryMsg::Bot { address } => to_json_binary(&query_bot(deps, address)?)?,
        QueryMsg::Bots {} => to_json_binary(&query_bots(deps)?)?,
        QueryMsg::Allowlist {} => to_json_binary(&query_allowlist(deps)?)?,
        QueryMsg::IsAllowlisted { bot } => to_json_binary(&query_is_allowlisted(deps, bot)?)?,
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

fn query_is_incentivized(
    deps: Deps,
    sender: String,
    rounds: Vec<u64>,
) -> StdResult<IsIncentivizedResponse> {
    let sender = deps.api.addr_validate(&sender)?;
    let config = CONFIG.load(deps.storage)?;
    let mut incentivized = Vec::<bool>::with_capacity(rounds.len());
    for round in rounds {
        incentivized.push(is_incentivized(deps, &sender, round, config.min_round)?);
    }
    Ok(IsIncentivizedResponse { incentivized })
}

/// Query submissions by round.
fn query_submissions(deps: Deps, round: u64) -> StdResult<SubmissionsResponse> {
    let prefix = SUBMISSIONS.prefix(round);

    let mut submissions: Vec<_> = prefix
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<Result<_, _>>()?;
    submissions.sort_by(|a, b| a.1.pos.cmp(&b.1.pos));

    let submissions = submissions
        .into_iter()
        .map(|(addr, stored)| QueriedSubmission::make(stored, addr))
        .collect();

    Ok(SubmissionsResponse { round, submissions })
}

fn query_bot(deps: Deps, address: String) -> StdResult<BotResponse> {
    let address = deps.api.addr_validate(&address)?;
    let bot = BOTS
        .may_load(deps.storage, &address)?
        .map(|bot| QueriedBot::make(bot, address));
    Ok(BotResponse { bot })
}

fn query_bots(deps: Deps) -> StdResult<BotsResponse> {
    // No pagination here yet 🤷‍♂️
    let bots = BOTS
        .range(deps.storage, None, None, Order::Ascending)
        .map(|result| {
            let (address, bot) = result.unwrap();
            QueriedBot::make(bot, address)
        })
        .collect();
    Ok(BotsResponse { bots })
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

fn query_is_allowlisted(deps: Deps, bot: String) -> StdResult<IsAllowlistedResponse> {
    let bot_addr = deps.api.addr_validate(&bot)?;
    let listed = ALLOWLIST.has(deps.storage, &bot_addr);
    Ok(IsAllowlistedResponse { listed })
}

fn execute_register_bot(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    moniker: String,
) -> Result<Response, ContractError> {
    validate_moniker(&moniker)?;
    let bot = match BOTS.may_load(deps.storage, &info.sender)? {
        Some(mut bot) => {
            bot.moniker = moniker;
            bot
        }
        _ => Bot {
            moniker,
            rounds_added: 0,
            reward_points: 0,
        },
    };
    BOTS.save(deps.storage, &info.sender, &bot)?;
    Ok(Response::default())
}

fn execute_set_incentivized(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    round: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    ensure_eq!(
        Some(info.sender),
        config.gateway,
        ContractError::Unauthorized
    );
    let min_round = config.min_round;
    if round < min_round {
        return Err(ContractError::RoundTooLow { round, min_round });
    }
    INCENTIVIZED_BY_GATEWAY.save(deps.storage, round, &INCENTIVIZED_BY_GATEWAY_MARKER)?;
    Ok(Response::default())
}

fn execute_update_allowlist_bots(
    deps: DepsMut,
    info: MessageInfo,
    add: Vec<String>,
    remove: Vec<String>,
) -> Result<Response, ContractError> {
    // check the calling address is the authorised multisig
    let config = CONFIG.load(deps.storage)?;
    ensure_eq!(info.sender, config.manager, ContractError::Unauthorized);

    // We add first to ensure an address that is included in both lists
    // is removed and not added.
    for bot in add {
        let addr = deps.api.addr_validate(&bot)?;
        if !ALLOWLIST.has(deps.storage, &addr) {
            ALLOWLIST.save(deps.storage, &addr, &())?;
        }
    }

    for bot in remove {
        let addr = deps.api.addr_validate(&bot)?;
        if ALLOWLIST.has(deps.storage, &addr) {
            ALLOWLIST.remove(deps.storage, &addr);
        }
    }

    Ok(Response::default())
}

/// This function submits the randomness from the bot to nois chain
/// It also incentivises the bots based on 3 criteria (computed BLS verification or not, Speed, processed callback jobs )
fn execute_add_round(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    network: Option<String>,
    round: u64,
    signature: HexBinary,
) -> Result<Response, ContractError> {
    // Handle sender is not sending funds
    // TODO: Not covered by testing
    if !info.funds.is_empty() {
        return Err(StdError::generic_err("Do not send funds").into());
    }

    let config = CONFIG.load(deps.storage)?;
    let min_round = config.min_round;
    if round < min_round {
        return Err(ContractError::RoundTooLow { round, min_round });
    }

    let network = DrandNetwork::from_str(network.as_deref().unwrap_or("fastnet"))?;

    // Initialise the incentive to 0
    let mut reward_points = 0u64;

    // Get the number of submission before this one.
    let submissions_count = match network {
        DrandNetwork::Fastnet => FASTNET_SUBMISSIONS_COUNT
            .may_load(deps.storage, round)?
            .unwrap_or_default(),
        DrandNetwork::Quicknet => QUICKNET_SUBMISSIONS_COUNT
            .may_load(deps.storage, round)?
            .unwrap_or_default(),
    };

    let randomness: HexBinary = derive_randomness(signature.as_slice()).into();
    // Check if we need to verify the submission  or we just compare it to the registered randomness from the first submission of this round
    let is_verifying_tx: bool;

    if submissions_count < NUMBER_OF_SUBMISSION_VERIFICATION_PER_ROUND {
        is_verifying_tx = true;

        match network {
            DrandNetwork::Fastnet => {
                // Since we have a static pubkey, it is safe to use the unchecked method
                let pk = G2PubkeyFastnet::from_fixed_unchecked(DrandNetwork::Fastnet.pubkey())
                    .map_err(|_| ContractError::InvalidPubkey {})?;
                // Verify BLS
                if !pk.verify(round, b"", &signature).unwrap_or(false) {
                    return Err(ContractError::InvalidSignature {});
                }
            }
            DrandNetwork::Quicknet => {
                // Since we have a static pubkey, it is safe to use the unchecked method
                let pk = G2PubkeyRfc::from_fixed_unchecked(DrandNetwork::Quicknet.pubkey())
                    .map_err(|_| ContractError::InvalidPubkey {})?;
                // Verify BLS
                if !pk.verify(round, b"", &signature).unwrap_or(false) {
                    return Err(ContractError::InvalidSignature {});
                }
            }
        }

        // Send verification reward
        reward_points += INCENTIVE_POINTS_FOR_VERIFICATION;
    } else {
        is_verifying_tx = false;
        //Check that the submitted randomness for the round is the same as the one verified in the state by the first submission tx
        //If the randomness is different error contract
        let already_verified_randomness_for_this_round =
            BEACONS.load(deps.storage, round)?.randomness;
        // Security wise the following check is not very necessary because this randomness is not going to be saved on state anyways as it is not the first submission of the round
        // Submitting here a wrong previous_signature will still make the contract pass but the randomness won't be persisted to contract.
        if randomness != already_verified_randomness_for_this_round {
            return Err(ContractError::SignatureDoesNotMatchState {});
        }
    }

    // Check if the bot is fast enough to get an incentive
    if submissions_count < NUMBER_OF_INCENTIVES_PER_ROUND {
        reward_points += INCENTIVE_POINTS_FOR_FAST_BOT;
    }

    let beacon = &VerifiedBeacon {
        verified: env.block.time,
        randomness: randomness.clone(),
    };

    let submissions_key = (round, &info.sender);

    if SUBMISSIONS.has(deps.storage, submissions_key) {
        return Err(ContractError::SubmissionExists);
    }

    let bot = BOTS.may_load(deps.storage, &info.sender)?;

    // True if and only if bot has been registered before
    let is_registered = bot.is_some();

    let is_allowlisted = ALLOWLIST.has(deps.storage, &info.sender);

    let new_count = submissions_count + 1;

    SUBMISSIONS.save(
        deps.storage,
        submissions_key,
        &StoredSubmission {
            pos: new_count,
            time: env.block.time,
            height: env.block.height,
            tx_index: env.transaction.map(|ti| ti.index),
        },
    )?;

    FASTNET_SUBMISSIONS_COUNT.save(deps.storage, round, &new_count)?;

    let mut attributes = vec![
        Attribute::new(ATTR_ROUND, round.to_string()),
        Attribute::new(ATTR_RANDOMNESS, randomness.to_hex()),
        Attribute::new(ATTR_BOT, info.sender.to_string()),
    ];

    // Execute the callback jobs and incentivise the drand bot based on howmany jobs they process

    let mut out_msgs = Vec::<CosmosMsg>::new();
    if let Some(gateway) = config.gateway {
        out_msgs.push(
            WasmMsg::Execute {
                contract_addr: gateway.into(),
                msg: to_json_binary(&NoisGatewayExecuteMsg::AddVerifiedRound {
                    round,
                    randomness,
                    is_verifying_tx,
                })?,
                funds: vec![],
            }
            .into(),
        );

        // TODO incentivise on processed_jobs;
    }

    // Pay the bot incentive
    // For now a bot needs to be registered, allowlisted and fast to  get incentives.
    // We can easily make unregistered bots eligible for incentives as well by changing
    // the following line

    let is_eligible = is_incentivized(deps.as_ref(), &info.sender, round, config.min_round)?
        && is_registered
        && is_allowlisted
        && reward_points != 0; // Allowed and registered bot that gathered reward points get incentives

    if !is_eligible {
        reward_points = 0;
    }

    attributes.push(Attribute::new(
        ATTR_REWARD_POINTS,
        reward_points.to_string(),
    ));

    if let Some(mut bot) = bot {
        bot.rounds_added += 1;
        bot.reward_points += reward_points;
        BOTS.save(deps.storage, &info.sender, &bot)?;
    }

    let payout = if is_eligible {
        let desired_amount = Uint128::from(reward_points) * config.incentive_point_price;

        let contract_balance = deps
            .querier
            .query_balance(&env.contract.address, &config.incentive_denom)?
            .amount;

        // The amount we'll actually pay out
        if contract_balance >= desired_amount {
            Coin {
                amount: desired_amount,
                denom: config.incentive_denom,
            }
        } else {
            Coin::new(0, config.incentive_denom)
        }
    } else {
        Coin::new(0, config.incentive_denom)
    };

    attributes.push(Attribute::new(ATTR_REWARD_PAYOUT, payout.to_string()));
    if !payout.amount.is_zero() {
        out_msgs.push(
            BankMsg::Send {
                to_address: info.sender.to_string(),
                amount: vec![payout],
            }
            .into(),
        );
    }

    if !BEACONS.has(deps.storage, round) {
        // Round is new
        BEACONS.save(deps.storage, round, beacon)?;
    } else {
        // Round has already been verified and must not be overriden to not
        // get a wrong `verified` timestamp.
    }

    Ok(Response::new()
        .add_messages(out_msgs)
        .add_attributes(attributes))
}

fn execute_set_config(
    deps: DepsMut,
    info: MessageInfo,
    manager: Option<String>,
    gateway: Option<String>,
    min_round: Option<u64>,
    incentive_point_price: Option<Uint128>,
    incentive_denom: Option<String>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // check the calling address is the authorised multisig
    ensure_eq!(info.sender, config.manager, ContractError::Unauthorized);

    let gateway = match gateway {
        Some(gateway) => Some(deps.api.addr_validate(&gateway)?),
        None => config.gateway,
    };
    let manager = match manager {
        Some(ma) => deps.api.addr_validate(&ma)?,
        None => config.manager,
    };
    let min_round = min_round.unwrap_or(config.min_round);
    let incentive_point_price = incentive_point_price.unwrap_or(config.incentive_point_price);
    let incentive_denom = incentive_denom.unwrap_or(config.incentive_denom);

    let new_config = Config {
        manager,
        gateway,
        min_round,
        incentive_point_price,
        incentive_denom,
    };

    CONFIG.save(deps.storage, &new_config)?;

    Ok(Response::default())
}

/// Returns true if this round is incentivized for the given `sender`.
/// Being incentivized for the bot is a basic property of a round a bot
/// should check. However, it does not guarantee an incentive. Further checks
/// like bot registration and allowlisting, available balance and order of
/// submissions are applied after that.
fn is_incentivized(deps: Deps, sender: &Addr, round: u64, min_round: u64) -> StdResult<bool> {
    if round < min_round {
        return Ok(false);
    }

    if INCENTIVIZED_BY_GATEWAY.has(deps.storage, round) || drand_common::is_incentivized(round) {
        return Ok(group(sender) == eligible_group(round));
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::ExecuteMsg;

    use cosmwasm_std::testing::{
        mock_dependencies, mock_dependencies_with_balance, mock_env, mock_info,
    };
    use cosmwasm_std::{coins, from_json, Addr, Timestamp, Uint128};
    use drand_common::testing::{testing_signature_fastnet, testing_signature_quicknet};
    use drand_common::DrandNetwork::*;
    use hex_literal::hex;

    const TESTING_MANAGER: &str = "mngr";
    const GATEWAY: &str = "thegateway";
    const TESTING_MIN_ROUND: u64 = 72750;

    const DEFAULT_TIME: Timestamp = Timestamp::from_nanos(1_571_797_419_879_305_533);
    const DEFAULT_HEIGHT: u64 = 12345;
    const DEFAULT_TX_INDEX: Option<u32> = Some(3);

    fn make_add_round_msg(network: DrandNetwork, round: u64) -> ExecuteMsg {
        let signature = match network {
            Fastnet => testing_signature_fastnet(round),
            Quicknet => testing_signature_quicknet(round),
        };
        let Some(signature) = signature else {
            panic!("Test round {round} not set for {network}");
        };

        ExecuteMsg::AddRound {
            network: Some(network.to_string()),
            round,
            signature,
        }
    }

    /// Adds round 72750, 72775, 72800, 72825
    fn add_test_rounds(mut deps: DepsMut, bot_addr: &str) {
        for round in [72750, 72775, 72800, 72825] {
            let msg = make_add_round_msg(Fastnet, round);
            execute(deps.branch(), mock_env(), mock_info(bot_addr, &[]), msg).unwrap();
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

    //
    // Instantiate tests
    //

    #[test]
    fn instantiate_works() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            manager: TESTING_MANAGER.to_string(),
            min_round: TESTING_MIN_ROUND,
            incentive_point_price: Uint128::new(20_000),
            incentive_denom: "unois".to_string(),
        };
        let info = mock_info("creator", &[]);
        let env = mock_env();
        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let config: ConfigResponse =
            from_json(query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
        assert_eq!(
            config,
            ConfigResponse {
                manager: Addr::unchecked(TESTING_MANAGER),
                gateway: None,
                min_round: TESTING_MIN_ROUND,
                incentive_point_price: Uint128::new(20_000),
                incentive_denom: "unois".to_string(),
            }
        );
    }

    //
    // Execute tests
    //
    fn register_bot(deps: DepsMut, sender_addr: impl AsRef<str>) {
        let info = mock_info(sender_addr.as_ref(), &[]);
        let register_bot_msg = ExecuteMsg::RegisterBot {
            moniker: "Best Bot".to_string(),
        };
        execute(deps, mock_env(), info, register_bot_msg).unwrap();
    }

    fn allowlist_bot(deps: DepsMut, addr: impl Into<String>) {
        let msg = ExecuteMsg::UpdateAllowlistBots {
            add: vec![addr.into()],
            remove: vec![],
        };
        execute(deps, mock_env(), mock_info(TESTING_MANAGER, &[]), msg).unwrap();
    }

    #[test]
    fn add_round_verifies_and_stores_randomness() {
        let mut deps = mock_dependencies();

        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            manager: TESTING_MANAGER.to_string(),
            min_round: TESTING_MIN_ROUND,
            incentive_point_price: Uint128::new(20_000),
            incentive_denom: "unois".to_string(),
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        register_bot(deps.as_mut(), "anyone");

        let msg = make_add_round_msg(Fastnet, 72780);
        let info = mock_info("anyone", &[]);
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let BeaconResponse { beacon } =
            from_json(query(deps.as_ref(), mock_env(), QueryMsg::Beacon { round: 72780 }).unwrap())
                .unwrap();
        assert_eq!(
            beacon.unwrap().randomness.to_hex(),
            "2f3a6976baf6847d75b5eae60c0e460bb55ab6034ee28aef2f0d10b0b5cc57c1"
        );
    }

    #[test]
    fn add_round_works_for_quicknet() {
        let mut deps = mock_dependencies();

        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            manager: TESTING_MANAGER.to_string(),
            min_round: TESTING_MIN_ROUND,
            incentive_point_price: Uint128::new(20_000),
            incentive_denom: "unois".to_string(),
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        register_bot(deps.as_mut(), "anyone");

        // https://api3.drand.sh/52db9ba70e0cc0f6eaf7803dd07447a1f5477735fd3f661792ba94600c84e971/public/7307065
        let msg = ExecuteMsg::AddRound {
            network: Some("quicknet".to_string()),
            round: 7307065,
            signature: HexBinary::from_hex("a8f51ba7ab91c937aa8c54f58da04172a4a9a8af2d7c95502e99c1dc29e574968b4b832446a18c251e07f7abc16a362f").unwrap(),
        };
        let expected_randomness =
            hex!("41155d35838a37100a908a4a713c593573d7ab6e845189fb24f7e31d72fc05e3");
        let info = mock_info("anyone", &[]);
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let BeaconResponse { beacon } = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Beacon { round: 7307065 },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(beacon.unwrap().randomness, &expected_randomness);
    }

    #[test]
    fn add_round_not_divisible_by_15_succeeds() {
        let mut deps = mock_dependencies_with_balance(&[Coin::new(9999999, "unois")]);

        let msg = InstantiateMsg {
            manager: TESTING_MANAGER.to_string(),
            min_round: TESTING_MIN_ROUND,
            incentive_point_price: Uint128::new(20_000),
            incentive_denom: "unois".to_string(),
        };
        let info = mock_info("creator", &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.messages.len(), 0);

        const ROUND: u64 = 72762; // https://api3.drand.sh/dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493/public/72762
        const EXPECTED_RANDOMNESS: &str =
            "528bb3e953412bea63dff0f86ad44aeda64953cd50040279e7b1aba4b9196f28";
        const BOT1: &str = "mr_bot";
        const BOT2: &str = "mrs_bot";

        register_bot(deps.as_mut(), BOT1);
        register_bot(deps.as_mut(), BOT2);
        allowlist_bot(deps.as_mut(), BOT1);
        allowlist_bot(deps.as_mut(), BOT2);

        // succeeds but not incentivized
        let msg = make_add_round_msg(Fastnet, ROUND);
        let response: Response =
            execute(deps.as_mut(), mock_env(), mock_info(BOT1, &[]), msg).unwrap();
        assert_eq!(response.messages.len(), 0);
        let attrs = response.attributes;
        let randomness = first_attr(&attrs, "randomness").unwrap();
        assert_eq!(randomness, EXPECTED_RANDOMNESS);
        assert_eq!(first_attr(&attrs, "reward_points").unwrap(), "0");
        assert_eq!(first_attr(&attrs, "reward_payout").unwrap(), "0unois");

        // Set gateway
        let msg = ExecuteMsg::SetConfig {
            manager: None,
            gateway: Some(GATEWAY.to_string()),
            min_round: None,
            incentive_point_price: None,
            incentive_denom: None,
        };
        let _res = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(TESTING_MANAGER, &[]),
            msg,
        )
        .unwrap();

        // Set incentivized
        let msg = ExecuteMsg::SetIncentivized { round: ROUND };
        let _response: Response =
            execute(deps.as_mut(), mock_env(), mock_info(GATEWAY, &[]), msg).unwrap();

        // succeeds and incentivized
        let msg = make_add_round_msg(Fastnet, ROUND);
        let response: Response =
            execute(deps.as_mut(), mock_env(), mock_info(BOT2, &[]), msg).unwrap();
        assert_eq!(response.messages.len(), 2);
        assert!(matches!(response.messages[0].msg, CosmosMsg::Wasm(_)));
        assert!(matches!(
            response.messages[1].msg,
            CosmosMsg::Bank(BankMsg::Send {
                to_address: _,
                amount: _
            })
        ));
        let attrs = response.attributes;
        let randomness = first_attr(&attrs, "randomness").unwrap();
        assert_eq!(randomness, EXPECTED_RANDOMNESS);
        assert_eq!(first_attr(&attrs, "reward_points").unwrap(), "50");
        assert_eq!(first_attr(&attrs, "reward_payout").unwrap(), "1000000unois");
    }

    #[test]
    fn add_round_fails_when_round_too_low() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            manager: TESTING_MANAGER.to_string(),
            min_round: TESTING_MIN_ROUND,
            incentive_point_price: Uint128::new(20_000),
            incentive_denom: "unois".to_string(),
        };
        let info = mock_info("creator", &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let ConfigResponse { min_round, .. } =
            from_json(query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
        assert_eq!(min_round, TESTING_MIN_ROUND);

        let msg = make_add_round_msg(Fastnet, 10);
        let err = execute(deps.as_mut(), mock_env(), mock_info("anyone", &[]), msg).unwrap_err();
        assert!(matches!(
            err,
            ContractError::RoundTooLow {
                round: 10,
                min_round: TESTING_MIN_ROUND,
            }
        ));
    }

    #[test]
    fn unregistered_bot_does_not_get_incentives() {
        let mut deps = mock_dependencies();

        let info = mock_info("creator", &[]);

        let env = mock_env();
        let contract = env.contract.address;
        //add balance to this contract
        deps.querier.update_balance(
            contract,
            vec![Coin {
                denom: "unois".to_string(),
                amount: Uint128::new(100_000_000),
            }],
        );

        let msg = InstantiateMsg {
            manager: TESTING_MANAGER.to_string(),
            min_round: TESTING_MIN_ROUND,
            incentive_point_price: Uint128::new(20_000),
            incentive_denom: "unois".to_string(),
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        const ROUND: u64 = 72780;
        const EXPECTED_RANDOMNESS: &str =
            "2f3a6976baf6847d75b5eae60c0e460bb55ab6034ee28aef2f0d10b0b5cc57c1";

        let msg = make_add_round_msg(Fastnet, ROUND);
        let info = mock_info("unregistered_bot", &[]);
        let response = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let attrs = response.attributes;
        let randomness = first_attr(&attrs, "randomness").unwrap();
        assert_eq!(randomness, EXPECTED_RANDOMNESS);
        assert_eq!(response.messages.len(), 0);
        assert_eq!(first_attr(&attrs, "reward_points").unwrap(), "0");
        assert_eq!(first_attr(&attrs, "reward_payout").unwrap(), "0unois");
    }

    #[test]
    fn set_incentivized_works() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            manager: TESTING_MANAGER.to_string(),
            min_round: TESTING_MIN_ROUND,
            incentive_point_price: Uint128::new(20_000),
            incentive_denom: "unois".to_string(),
        };
        let info = mock_info("creator", &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let ConfigResponse { min_round, .. } =
            from_json(query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
        assert_eq!(min_round, TESTING_MIN_ROUND);

        const GOOD_ROUND: u64 = TESTING_MIN_ROUND + 10;
        const BAD_ROUND: u64 = TESTING_MIN_ROUND - 10;

        // Gateway not set, unauthorized
        let msg = ExecuteMsg::SetIncentivized { round: GOOD_ROUND };
        let err = execute(deps.as_mut(), mock_env(), mock_info(GATEWAY, &[]), msg).unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized {}));

        // Set gateway
        let msg = ExecuteMsg::SetConfig {
            manager: None,
            gateway: Some(GATEWAY.to_string()),
            min_round: None,
            incentive_point_price: None,
            incentive_denom: None,
        };
        let _res = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(TESTING_MANAGER, &[]),
            msg,
        )
        .unwrap();

        // Gateway can set now
        let msg = ExecuteMsg::SetIncentivized { round: GOOD_ROUND };
        let _res = execute(deps.as_mut(), mock_env(), mock_info(GATEWAY, &[]), msg).unwrap();

        // Someone else is unauthoized
        let msg = ExecuteMsg::SetIncentivized { round: GOOD_ROUND };
        let err = execute(deps.as_mut(), mock_env(), mock_info("anyone", &[]), msg).unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized {}));

        let msg = ExecuteMsg::SetIncentivized { round: BAD_ROUND };
        let err = execute(deps.as_mut(), mock_env(), mock_info(GATEWAY, &[]), msg).unwrap_err();
        assert!(matches!(
            err,
            ContractError::RoundTooLow {
                round: BAD_ROUND,
                min_round: TESTING_MIN_ROUND,
            }
        ));
    }

    #[test]
    fn allowlisting_and_deallowlisting_work() {
        // First we will register a bot
        // Then check that the bot doesnt get incentives by submitting
        // Then add the bot to the allowlist and check that this time it gets incentives
        // Then deallowlist the bot and make sure it doesnt get incentives anymore
        // Note that we need submit different randomness rounds each time
        // because the same bot operator is not allowed to submit the same randomness
        let mut deps = mock_dependencies();

        const BOT: &str = "bobbybot";

        let msg = InstantiateMsg {
            manager: TESTING_MANAGER.to_string(),
            min_round: TESTING_MIN_ROUND,
            incentive_point_price: Uint128::new(20_000),
            incentive_denom: "unois".to_string(),
        };
        instantiate(deps.as_mut(), mock_env(), mock_info("creator", &[]), msg).unwrap();

        let IsAllowlistedResponse { listed } = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::IsAllowlisted {
                    bot: BOT.to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert!(!listed);

        // allowlist
        let msg = ExecuteMsg::UpdateAllowlistBots {
            add: vec![BOT.to_string()],
            remove: vec![],
        };
        let info = mock_info(TESTING_MANAGER, &[]);
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // adding same address again is fine
        let msg = ExecuteMsg::UpdateAllowlistBots {
            add: vec![BOT.to_string()],
            remove: vec![],
        };
        let info = mock_info(TESTING_MANAGER, &[]);
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let IsAllowlistedResponse { listed } = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::IsAllowlisted {
                    bot: BOT.to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert!(listed);

        // deallowlist
        let msg = ExecuteMsg::UpdateAllowlistBots {
            add: vec![],
            remove: vec![BOT.to_string()],
        };
        let info = mock_info(TESTING_MANAGER, &[]);
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let IsAllowlistedResponse { listed } = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::IsAllowlisted {
                    bot: BOT.to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert!(!listed);

        // removal takes precendence over additions (better safe than sorry)
        let msg = ExecuteMsg::UpdateAllowlistBots {
            add: vec![BOT.to_string()],
            remove: vec![BOT.to_string()],
        };
        let info = mock_info(TESTING_MANAGER, &[]);
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let IsAllowlistedResponse { listed } = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::IsAllowlisted {
                    bot: BOT.to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert!(!listed);
    }

    #[test]
    fn updateallowlistbots_handles_invalid_addresses() {
        let mut deps = mock_dependencies();

        let info = mock_info("creator", &[]);

        let msg = InstantiateMsg {
            manager: TESTING_MANAGER.to_string(),
            min_round: TESTING_MIN_ROUND,
            incentive_point_price: Uint128::new(20_000),
            incentive_denom: "unois".to_string(),
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // add: Empty value
        let msg = ExecuteMsg::UpdateAllowlistBots {
            add: vec!["".to_string()],
            remove: vec![],
        };
        let info = mock_info(TESTING_MANAGER, &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        match err {
            ContractError::Std(StdError::GenericErr { msg }) => {
                assert_eq!(msg, "Invalid input: human address too short for this mock implementation (must be >= 3).")
            }
            _ => panic!("Unexpected error: {err:?}"),
        }

        // add: Upper case address
        let msg = ExecuteMsg::UpdateAllowlistBots {
            add: vec!["theADDRESS".to_string()],
            remove: vec![],
        };
        let info = mock_info(TESTING_MANAGER, &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        match err {
            ContractError::Std(StdError::GenericErr { msg }) => {
                assert_eq!(msg, "Invalid input: address not normalized")
            }
            _ => panic!("Unexpected error: {err:?}"),
        }

        // remove: Empty value
        let msg = ExecuteMsg::UpdateAllowlistBots {
            add: vec![],
            remove: vec!["".to_string()],
        };
        let info = mock_info(TESTING_MANAGER, &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        match err {
            ContractError::Std(StdError::GenericErr { msg }) => {
                assert_eq!(msg, "Invalid input: human address too short for this mock implementation (must be >= 3).")
            }
            _ => panic!("Unexpected error: {err:?}"),
        }

        // remove: Upper case address
        let msg = ExecuteMsg::UpdateAllowlistBots {
            add: vec![],
            remove: vec!["theADDRESS".to_string()],
        };
        let info = mock_info(TESTING_MANAGER, &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        match err {
            ContractError::Std(StdError::GenericErr { msg }) => {
                assert_eq!(msg, "Invalid input: address not normalized")
            }
            _ => panic!("Unexpected error: {err:?}"),
        }
    }

    #[test]
    fn when_contract_does_not_have_enough_funds_no_bot_incentives_are_sent() {
        let mut deps = mock_dependencies();

        let info = mock_info("creator", &[]);

        let env = mock_env();
        let contract = env.contract.address;
        //add balance to the contract
        deps.querier
            .update_balance(contract, vec![Coin::new(10_000, "unois")]);

        let msg = InstantiateMsg {
            manager: TESTING_MANAGER.to_string(),
            min_round: TESTING_MIN_ROUND,
            incentive_point_price: Uint128::new(20_000),
            incentive_denom: "unois".to_string(),
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        const MYBOT: &str = "mybot_12"; // eligable for odd rounds

        register_bot(deps.as_mut(), MYBOT);
        allowlist_bot(deps.as_mut(), MYBOT);

        const ROUND: u64 = 72775;
        const EXPECTED_RANDOMNESS: &str =
            "b84506d4342f4ec2506baa60a6b611ab006cf45e870d069ebb1b6a051c9e9acf";

        let msg = make_add_round_msg(Fastnet, ROUND);
        let info = mock_info(MYBOT, &[]);
        let response = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(response.messages.len(), 0);
        let attrs = response.attributes;
        assert_eq!(
            first_attr(&attrs, "randomness").unwrap(),
            EXPECTED_RANDOMNESS
        );
        assert_eq!(first_attr(&attrs, "reward_points").unwrap(), "50");
        assert_eq!(first_attr(&attrs, "reward_payout").unwrap(), "0unois");
    }

    #[test]
    fn only_top_x_bots_receive_incentive() {
        let mut deps = mock_dependencies();

        let info = mock_info("creator", &[Coin::new(100_000_000, "unois")]);
        let env = mock_env();
        let contract = env.contract.address;
        // add balance to the drand contract
        deps.querier.update_balance(
            contract,
            vec![Coin {
                denom: "unois".to_string(),
                amount: Uint128::new(100_000_000),
            }],
        );

        let msg = InstantiateMsg {
            manager: TESTING_MANAGER.to_string(),
            min_round: TESTING_MIN_ROUND,
            incentive_point_price: Uint128::new(20_000),
            incentive_denom: "unois".to_string(),
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let bot1 = "registered_bot_761826381";
        let bot2 = "registered_bot_98787233";
        let bot3 = "registered_bot_12618926371";
        let bot4 = "registered_bot_21739812";
        let bot5 = "registered_bot_26737162";
        let bot6 = "registered_bot_34216397";
        let bot7 = "registered_bot_0821738";

        register_bot(deps.as_mut(), bot1);
        register_bot(deps.as_mut(), bot2);
        register_bot(deps.as_mut(), bot3);
        register_bot(deps.as_mut(), bot4);
        register_bot(deps.as_mut(), bot5);
        register_bot(deps.as_mut(), bot6);
        register_bot(deps.as_mut(), bot7);

        // add bots to allowlist
        let msg = ExecuteMsg::UpdateAllowlistBots {
            add: vec![
                bot1.to_string(),
                bot2.to_string(),
                bot3.to_string(),
                bot4.to_string(),
                bot5.to_string(),
                bot6.to_string(),
                bot7.to_string(),
            ],
            remove: vec![],
        };
        let info = mock_info(TESTING_MANAGER, &[]);
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Same msg for all submissions
        const ROUND: u64 = 72775;
        let msg = make_add_round_msg(Fastnet, ROUND);

        // 1st
        let info = mock_info(bot1, &[]);
        let response = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();
        assert_eq!(response.messages.len(), 1);
        assert_eq!(
            response.messages[0].msg,
            CosmosMsg::Bank(BankMsg::Send {
                to_address: "registered_bot_761826381".to_string(),
                amount: coins(1000000, "unois"), // verification + fast
            })
        );

        // 2nd
        let info = mock_info(bot2, &[]);
        let response = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();
        assert_eq!(response.messages.len(), 1);
        assert_eq!(
            response.messages[0].msg,
            CosmosMsg::Bank(BankMsg::Send {
                to_address: "registered_bot_98787233".to_string(),
                amount: coins(1000000, "unois"), // verification + fast
            })
        );

        // 3rd
        let info = mock_info(bot3, &[]);
        let response = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();
        assert_eq!(response.messages.len(), 1);
        assert_eq!(
            response.messages[0].msg,
            CosmosMsg::Bank(BankMsg::Send {
                to_address: "registered_bot_12618926371".to_string(),
                amount: coins(1000000, "unois"), // verification + fast
            })
        );

        // 4th
        let info = mock_info(bot4, &[]);
        let response = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();
        assert_eq!(response.messages.len(), 1);
        assert_eq!(
            response.messages[0].msg,
            CosmosMsg::Bank(BankMsg::Send {
                to_address: "registered_bot_21739812".to_string(),
                amount: coins(300_000, "unois"), // fast, no verification
            })
        );

        // 5th
        let info = mock_info(bot5, &[]);
        let response = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();
        assert_eq!(response.messages.len(), 1);
        assert_eq!(
            response.messages[0].msg,
            CosmosMsg::Bank(BankMsg::Send {
                to_address: "registered_bot_26737162".to_string(),
                amount: coins(300_000, "unois"), // fast, no verification
            })
        );

        // 6th
        let info = mock_info(bot6, &[]);
        let response = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();
        assert_eq!(response.messages.len(), 1);
        assert_eq!(
            response.messages[0].msg,
            CosmosMsg::Bank(BankMsg::Send {
                to_address: "registered_bot_34216397".to_string(),
                amount: coins(300_000, "unois"), // fast, no verification
            })
        );

        // 7th, here no message is emitted
        let info = mock_info(bot7, &[]);
        let response = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(response.messages.len(), 0);
    }

    #[test]
    fn unregistered_bot_can_add_round() {
        let mut deps = mock_dependencies();

        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            manager: TESTING_MANAGER.to_string(),
            min_round: TESTING_MIN_ROUND,
            incentive_point_price: Uint128::new(20_000),
            incentive_denom: "unois".to_string(),
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg = make_add_round_msg(Fastnet, 72780);
        let info = mock_info("unregistered_bot", &[]);
        let response = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let randomness = first_attr(response.attributes, "randomness").unwrap();
        assert_eq!(
            randomness,
            "2f3a6976baf6847d75b5eae60c0e460bb55ab6034ee28aef2f0d10b0b5cc57c1"
        );
    }

    #[test]
    fn add_round_fails_for_broken_signature() {
        let mut deps = mock_dependencies();

        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            manager: TESTING_MANAGER.to_string(),
            min_round: TESTING_MIN_ROUND,
            incentive_point_price: Uint128::new(20_000),
            incentive_denom: "unois".to_string(),
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        register_bot(deps.as_mut(), "anyone");
        let info = mock_info("anyone", &[]);
        let msg = ExecuteMsg::AddRound {
            network: Some("fastnet".to_string()),
            round: 72780,
            signature: hex::decode("3cc6f6cdf59e95526d5a5d82aaa84fa6f181e4")
                .unwrap()
                .into(), // broken signature
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
        let msg = InstantiateMsg {
            manager: TESTING_MANAGER.to_string(),
            min_round: TESTING_MIN_ROUND,
            incentive_point_price: Uint128::new(20_000),
            incentive_denom: "unois".to_string(),
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg = ExecuteMsg::AddRound {
            network: Some("fastnet".to_string()),
            round: 72790, // wrong round
            signature: testing_signature_fastnet(72780).unwrap(),
        };
        let result = execute(deps.as_mut(), mock_env(), mock_info("anon", &[]), msg);
        match result.unwrap_err() {
            ContractError::InvalidSignature {} => {}
            err => panic!("Unexpected error: {:?}", err),
        };

        let msg = ExecuteMsg::AddRound {
            // curl -sS https://drand.cloudflare.com/dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493/public/72780
            network: Some("fastnet".to_string()),
            round: 72780,
            // wrong signature (first two bytes swapped)
            signature: hex::decode("ac86005aaffa5e9de34b558c470a111c862e976922e8da34f9dce1a78507dbd53badd554862bc54bd8e44f44ddd8b100").unwrap().into(),
        };
        let result = execute(deps.as_mut(), mock_env(), mock_info("anon", &[]), msg);
        match result.unwrap_err() {
            ContractError::InvalidSignature {} => {}
            err => panic!("Unexpected error: {:?}", err),
        };
    }

    #[test]
    fn add_round_succeeds_multiple_times() {
        let mut deps = mock_dependencies();

        register_bot(deps.as_mut(), "creator");

        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            manager: TESTING_MANAGER.to_string(),
            min_round: TESTING_MIN_ROUND,
            incentive_point_price: Uint128::new(20_000),
            incentive_denom: "unois".to_string(),
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg = make_add_round_msg(Fastnet, 72780);

        // Execute 1
        register_bot(deps.as_mut(), "anyone");
        let info = mock_info("anyone", &[]);
        let response = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();
        let randomness = first_attr(response.attributes, "randomness").unwrap();
        assert_eq!(
            randomness,
            "2f3a6976baf6847d75b5eae60c0e460bb55ab6034ee28aef2f0d10b0b5cc57c1"
        );

        // Execute 2
        register_bot(deps.as_mut(), "someone else");
        let info = mock_info("someone else", &[]);
        let response = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let randomness = first_attr(response.attributes, "randomness").unwrap();
        assert_eq!(
            randomness,
            "2f3a6976baf6847d75b5eae60c0e460bb55ab6034ee28aef2f0d10b0b5cc57c1"
        );
    }

    #[test]
    fn add_round_fails_when_same_bot_submits_multiple_times() {
        let mut deps = mock_dependencies();

        register_bot(deps.as_mut(), "creator");

        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            manager: TESTING_MANAGER.to_string(),
            min_round: TESTING_MIN_ROUND,
            incentive_point_price: Uint128::new(20_000),
            incentive_denom: "unois".to_string(),
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg = make_add_round_msg(Fastnet, 72780);

        const A: &str = "bot_alice";
        const B: &str = "bot_bob";
        register_bot(deps.as_mut(), A);
        register_bot(deps.as_mut(), B);

        // Execute A1
        let info = mock_info(A, &[]);
        execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();
        // Execute B1
        let info = mock_info(B, &[]);
        execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();

        // Execute A2
        let info = mock_info(A, &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
        assert!(matches!(err, ContractError::SubmissionExists));
        // Execute B2
        let info = mock_info(B, &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert!(matches!(err, ContractError::SubmissionExists));
    }

    #[test]
    fn register_bot_works_for_updates() {
        let mut deps = mock_dependencies();
        let bot_addr = "bot_addr".to_string();

        // first registration

        let info = mock_info(&bot_addr, &[]);
        let register_bot_msg = ExecuteMsg::RegisterBot {
            moniker: "Nickname1".to_string(),
        };
        execute(deps.as_mut(), mock_env(), info, register_bot_msg).unwrap();
        let BotResponse { bot } = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Bot {
                    address: bot_addr.clone(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        let bot = bot.unwrap();
        assert_eq!(
            bot,
            QueriedBot {
                moniker: "Nickname1".to_string(),
                address: Addr::unchecked(&bot_addr),
                rounds_added: 0,
                reward_points: 0,
            }
        );

        // re-register

        let info = mock_info(&bot_addr, &[]);
        let register_bot_msg = ExecuteMsg::RegisterBot {
            moniker: "Another nickname".to_string(),
        };
        execute(deps.as_mut(), mock_env(), info, register_bot_msg).unwrap();
        let BotResponse { bot } = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Bot {
                    address: bot_addr.clone(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        let bot = bot.unwrap();
        assert_eq!(
            bot,
            QueriedBot {
                moniker: "Another nickname".to_string(),
                address: Addr::unchecked(&bot_addr),
                rounds_added: 0,
                reward_points: 0,
            }
        );
    }

    //
    // Query tests
    //

    #[test]
    fn query_beacons_asc_works() {
        let mut deps = mock_dependencies();

        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            manager: TESTING_MANAGER.to_string(),
            min_round: TESTING_MIN_ROUND,
            incentive_point_price: Uint128::new(20_000),
            incentive_denom: "unois".to_string(),
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        register_bot(deps.as_mut(), "anyone");
        add_test_rounds(deps.as_mut(), "anyone");

        // Unlimited
        let BeaconsResponse { beacons } = from_json(
            query(
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
        assert_eq!(response_rounds, [72750, 72775, 72800, 72825]);

        // Limit 2
        let BeaconsResponse { beacons } = from_json(
            query(
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
        assert_eq!(response_rounds, [72750, 72775]);

        // After 0
        let BeaconsResponse { beacons } = from_json(
            query(
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
        assert_eq!(response_rounds, [72750, 72775, 72800, 72825]);

        // After 72760
        let BeaconsResponse { beacons } = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::BeaconsAsc {
                    start_after: Some(72760),
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();
        let response_rounds = beacons.iter().map(|b| b.round).collect::<Vec<u64>>();
        assert_eq!(response_rounds, [72775, 72800, 72825]);

        // After 72825
        let BeaconsResponse { beacons } = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::BeaconsAsc {
                    start_after: Some(72825),
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

        register_bot(deps.as_mut(), "creator");
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            manager: TESTING_MANAGER.to_string(),
            min_round: TESTING_MIN_ROUND,
            incentive_point_price: Uint128::new(20_000),
            incentive_denom: "unois".to_string(),
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        register_bot(deps.as_mut(), "anyone");
        add_test_rounds(deps.as_mut(), "anyone");

        // Unlimited
        let BeaconsResponse { beacons } = from_json(
            query(
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
        assert_eq!(response_rounds, [72825, 72800, 72775, 72750]);

        // Limit 2
        let BeaconsResponse { beacons } = from_json(
            query(
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
        assert_eq!(response_rounds, [72825, 72800]);

        // After 99999
        let BeaconsResponse { beacons } = from_json(
            query(
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
        assert_eq!(response_rounds, [72825, 72800, 72775, 72750]);

        // After 72780
        let BeaconsResponse { beacons } = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::BeaconsDesc {
                    start_after: Some(72780),
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();
        let response_rounds = beacons.iter().map(|b| b.round).collect::<Vec<u64>>();
        assert_eq!(response_rounds, [72775, 72750]);

        // After 72750
        let BeaconsResponse { beacons } = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::BeaconsDesc {
                    start_after: Some(72750),
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
    fn query_is_incentivized_works() {
        let mut deps = mock_dependencies();

        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            manager: TESTING_MANAGER.to_string(),
            min_round: TESTING_MIN_ROUND,
            incentive_point_price: Uint128::new(20_000),
            incentive_denom: "unois".to_string(),
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        const BOT1: &str = "beta1";
        const BOT2: &str = "gamma2";

        // No rounds
        let response: IsIncentivizedResponse = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::IsIncentivized {
                    sender: BOT1.to_string(),
                    rounds: vec![],
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(response.incentivized, Vec::<bool>::new());

        // One round (incentivized by round, not job)
        let response: IsIncentivizedResponse = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::IsIncentivized {
                    sender: BOT1.to_string(),
                    rounds: vec![TESTING_MIN_ROUND],
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(response.incentivized, vec![true]);

        let response: IsIncentivizedResponse = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::IsIncentivized {
                    sender: BOT1.to_string(),
                    rounds: vec![TESTING_MIN_ROUND + 15],
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(response.incentivized, vec![false]);

        // multiple
        let rounds = [
            TESTING_MIN_ROUND,
            TESTING_MIN_ROUND + 25,
            TESTING_MIN_ROUND + 50,
        ];
        let response: IsIncentivizedResponse = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::IsIncentivized {
                    sender: BOT1.to_string(),
                    rounds: rounds.into(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(response.incentivized, vec![true, false, true]);

        // bot 2 gets different results
        let response: IsIncentivizedResponse = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::IsIncentivized {
                    sender: BOT2.to_string(),
                    rounds: rounds.into(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(response.incentivized, vec![false, true, false]);

        // many
        let response: IsIncentivizedResponse = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::IsIncentivized {
                    sender: BOT1.to_string(),
                    rounds: vec![
                        TESTING_MIN_ROUND,
                        TESTING_MIN_ROUND + 10,
                        TESTING_MIN_ROUND + 11,
                        TESTING_MIN_ROUND + 12,
                        TESTING_MIN_ROUND + 13,
                        TESTING_MIN_ROUND + 14,
                        TESTING_MIN_ROUND + 15,
                        TESTING_MIN_ROUND + 16,
                        TESTING_MIN_ROUND + 17,
                        TESTING_MIN_ROUND + 18,
                        TESTING_MIN_ROUND + 19,
                        TESTING_MIN_ROUND + 20,
                        TESTING_MIN_ROUND + 21,
                    ],
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            response.incentivized,
            vec![
                true, false, false, false, false, false, false, false, false, false, false, false,
                false
            ]
        );

        // Very many. It should be up to the node operator to limit query cost using the gas settings.
        // There might be cases in which you want to mass query this.
        let response: IsIncentivizedResponse = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::IsIncentivized {
                    sender: BOT1.to_string(),
                    rounds: (TESTING_MIN_ROUND..TESTING_MIN_ROUND + 999).collect(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(response.incentivized.len(), 999);

        // Values below min_round are always false
        let response: IsIncentivizedResponse = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::IsIncentivized {
                    sender: BOT1.to_string(),
                    rounds: (TESTING_MIN_ROUND - 200..TESTING_MIN_ROUND).collect(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(response.incentivized, vec![false; 200]);
    }

    #[test]
    fn query_submissions_works() {
        let mut deps = mock_dependencies();

        register_bot(deps.as_mut(), "creator");
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            manager: TESTING_MANAGER.to_string(),
            min_round: TESTING_MIN_ROUND,
            incentive_point_price: Uint128::new(20_000),
            incentive_denom: "unois".to_string(),
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Address order is not submission order
        let bot1 = "beta1";
        let bot2 = "gamma2";
        let bot3 = "alpha3";

        register_bot(deps.as_mut(), bot1);
        add_test_rounds(deps.as_mut(), bot1);

        let test_round = 72775;

        // No submissions
        let response: SubmissionsResponse = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Submissions {
                    round: test_round - 1,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(response.round, test_round - 1);
        assert_eq!(response.submissions, Vec::<_>::new());

        // One submission
        let response: SubmissionsResponse = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Submissions { round: test_round },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(response.round, test_round);
        assert_eq!(
            response.submissions,
            [QueriedSubmission {
                bot: Addr::unchecked(bot1),
                time: DEFAULT_TIME,
                height: DEFAULT_HEIGHT,
                tx_index: DEFAULT_TX_INDEX,
            }]
        );

        add_test_rounds(deps.as_mut(), bot2);

        // Two submissions
        let response: SubmissionsResponse = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Submissions { round: test_round },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(response.round, test_round);
        assert_eq!(
            response.submissions,
            [
                QueriedSubmission {
                    bot: Addr::unchecked(bot1),
                    time: DEFAULT_TIME,
                    height: DEFAULT_HEIGHT,
                    tx_index: DEFAULT_TX_INDEX,
                },
                QueriedSubmission {
                    bot: Addr::unchecked(bot2),
                    time: DEFAULT_TIME,
                    height: DEFAULT_HEIGHT,
                    tx_index: DEFAULT_TX_INDEX,
                },
            ]
        );

        add_test_rounds(deps.as_mut(), bot3);

        // Three submissions
        let response: SubmissionsResponse = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Submissions { round: test_round },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(response.round, test_round);
        assert_eq!(
            response.submissions,
            [
                QueriedSubmission {
                    bot: Addr::unchecked(bot1),
                    time: DEFAULT_TIME,
                    height: DEFAULT_HEIGHT,
                    tx_index: DEFAULT_TX_INDEX,
                },
                QueriedSubmission {
                    bot: Addr::unchecked(bot2),
                    time: DEFAULT_TIME,
                    height: DEFAULT_HEIGHT,
                    tx_index: DEFAULT_TX_INDEX,
                },
                QueriedSubmission {
                    bot: Addr::unchecked(bot3),
                    time: DEFAULT_TIME,
                    height: DEFAULT_HEIGHT,
                    tx_index: DEFAULT_TX_INDEX,
                },
            ]
        );
    }

    #[test]
    fn query_allowlist_works() {
        let mut deps = mock_dependencies();

        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            manager: TESTING_MANAGER.to_string(),
            min_round: TESTING_MIN_ROUND,
            incentive_point_price: Uint128::new(20_000),
            incentive_denom: "unois".to_string(),
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let AllowlistResponse { allowed } =
            from_json(query(deps.as_ref(), mock_env(), QueryMsg::Allowlist {}).unwrap()).unwrap();
        assert_eq!(allowed, Vec::<String>::new());

        // Add one entry
        let info = mock_info(TESTING_MANAGER, &[]);
        execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::UpdateAllowlistBots {
                add: vec!["bot_b".to_string()],
                remove: vec![],
            },
        )
        .unwrap();

        let AllowlistResponse { allowed } =
            from_json(query(deps.as_ref(), mock_env(), QueryMsg::Allowlist {}).unwrap()).unwrap();
        assert_eq!(allowed, vec!["bot_b".to_string()]);

        // Add two more entries
        let info = mock_info(TESTING_MANAGER, &[]);
        execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::UpdateAllowlistBots {
                add: vec!["bot_a".to_string(), "bot_c".to_string()],
                remove: vec![],
            },
        )
        .unwrap();

        let AllowlistResponse { allowed } =
            from_json(query(deps.as_ref(), mock_env(), QueryMsg::Allowlist {}).unwrap()).unwrap();
        assert_eq!(
            allowed,
            vec![
                "bot_a".to_string(),
                "bot_b".to_string(),
                "bot_c".to_string()
            ]
        );
    }

    #[test]
    fn query_bot_works() {
        let mut deps = mock_dependencies();

        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            manager: TESTING_MANAGER.to_string(),
            min_round: TESTING_MIN_ROUND,
            incentive_point_price: Uint128::new(20_000),
            incentive_denom: "unois".to_string(),
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        const UNREGISTERED: &str = "unregistered_bot";
        const REGISTERED: &str = "registered_bot";
        const ALLOWLISTED: &str = "allowlisted_bot";

        register_bot(deps.as_mut(), REGISTERED);
        register_bot(deps.as_mut(), ALLOWLISTED);
        allowlist_bot(deps.as_mut(), ALLOWLISTED);

        // Unregisrered
        let BotResponse { bot } = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Bot {
                    address: UNREGISTERED.to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(bot, None);

        // Registered
        let BotResponse { bot } = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Bot {
                    address: REGISTERED.to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            bot.unwrap(),
            QueriedBot {
                address: Addr::unchecked(REGISTERED),
                moniker: "Best Bot".to_string(),
                rounds_added: 0,
                reward_points: 0,
            }
        );

        add_test_rounds(deps.as_mut(), REGISTERED);

        let BotResponse { bot } = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Bot {
                    address: REGISTERED.to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            bot.unwrap(),
            QueriedBot {
                address: Addr::unchecked(REGISTERED),
                moniker: "Best Bot".to_string(),
                rounds_added: 4,
                reward_points: 0, // Not allowlisted
            }
        );

        // Allowlisted
        let BotResponse { bot } = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Bot {
                    address: ALLOWLISTED.to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            bot.unwrap(),
            QueriedBot {
                address: Addr::unchecked(ALLOWLISTED),
                moniker: "Best Bot".to_string(),
                rounds_added: 0,
                reward_points: 0,
            }
        );

        add_test_rounds(deps.as_mut(), ALLOWLISTED);

        let BotResponse { bot } = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Bot {
                    address: ALLOWLISTED.to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            bot.unwrap(),
            QueriedBot {
                address: Addr::unchecked(ALLOWLISTED),
                moniker: "Best Bot".to_string(),
                rounds_added: 4,
                reward_points: 2
                    * (INCENTIVE_POINTS_FOR_FAST_BOT + INCENTIVE_POINTS_FOR_VERIFICATION),
            }
        );
    }

    #[test]
    fn is_query_allowlisted_works() {
        let mut deps = mock_dependencies();

        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            manager: TESTING_MANAGER.to_string(),
            min_round: TESTING_MIN_ROUND,
            incentive_point_price: Uint128::new(20_000),
            incentive_denom: "unois".to_string(),
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Add one entry
        let info = mock_info(TESTING_MANAGER, &[]);
        execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::UpdateAllowlistBots {
                add: vec!["bot_b".to_string()],
                remove: vec![],
            },
        )
        .unwrap();

        // bot_b is listed
        let IsAllowlistedResponse { listed } = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::IsAllowlisted {
                    bot: "bot_b".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert!(listed);

        // bot_a is not listed
        let IsAllowlistedResponse { listed } = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::IsAllowlisted {
                    bot: "bot_a".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert!(!listed);
    }

    #[test]
    fn only_manager_can_set_manager() {
        let mut deps = mock_dependencies();

        register_bot(deps.as_mut(), "creator");
        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            manager: TESTING_MANAGER.to_string(),
            min_round: TESTING_MIN_ROUND,
            incentive_point_price: Uint128::new(20_000),
            incentive_denom: "unois".to_string(),
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // A random addr cannot set a new manager
        let info = mock_info("some_random_person", &[]);
        let msg = ExecuteMsg::SetConfig {
            manager: Some("new_manager".to_string()),
            gateway: None,
            incentive_denom: None,
            incentive_point_price: None,
            min_round: None,
        };
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized));

        // Creator cannot set a new manager
        let info = mock_info("CREATOR", &[]);
        let msg = ExecuteMsg::SetConfig {
            manager: Some("new_manager".to_string()),
            gateway: None,
            incentive_denom: None,
            incentive_point_price: None,
            min_round: None,
        };

        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized));

        // Manager can set a new manager
        let info = mock_info(TESTING_MANAGER, &[]);
        let msg = ExecuteMsg::SetConfig {
            manager: Some("new_manager".to_string()),
            gateway: None,
            incentive_denom: None,
            incentive_point_price: None,
            min_round: None,
        };
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let config: ConfigResponse =
            from_json(query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
        assert_eq!(
            config,
            ConfigResponse {
                manager: Addr::unchecked("new_manager"),
                gateway: None,
                min_round: TESTING_MIN_ROUND,
                incentive_point_price: Uint128::new(20_000),
                incentive_denom: "unois".to_string(),
            }
        );
    }
}
