use cosmwasm_std::{
    ensure_eq, entry_point, to_binary, Addr, Attribute, BankMsg, Coin, CosmosMsg, Deps, DepsMut,
    Empty, Env, HexBinary, MessageInfo, Order, QueryResponse, Response, StdError, StdResult,
    Uint128, WasmMsg,
};
use cw_storage_plus::Bound;
use drand_verify::{derive_randomness, g1_from_fixed_unchecked, verify};

use crate::attributes::{
    ATTR_BOT, ATTR_RANDOMNESS, ATTR_REWARD_PAYOUT, ATTR_REWARD_POINTS, ATTR_ROUND,
};
use crate::bots::validate_moniker;
use crate::drand::DRAND_MAINNET_PUBKEY;
use crate::error::ContractError;
use crate::msg::{
    AllowListResponse, BeaconResponse, BeaconsResponse, BotResponse, BotsResponse, ConfigResponse,
    ExecuteMsg, InstantiateMsg, IsAllowListedResponse, NoisGatewayExecuteMsg, QueriedSubmission,
    QueryMsg, SubmissionsResponse,
};
use crate::state::{
    Bot, Config, QueriedBeacon, QueriedBot, StoredSubmission, VerifiedBeacon, ALLOWLIST, BEACONS,
    BOTS, CONFIG, SUBMISSIONS, SUBMISSIONS_ORDER,
};

/// Constant defining how many submissions per round will be rewarded
const NUMBER_OF_INCENTIVES_PER_ROUND: u32 = 6;
const NUMBER_OF_SUBMISSION_VERIFICATION_PER_ROUND: u32 = 3;
const INCENTIVE_POINTS_FOR_VERIFICATION: Uint128 = Uint128::new(35);
const INCENTIVE_POINTS_FOR_FAST_BOT: Uint128 = Uint128::new(15);

#[entry_point]
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
        ExecuteMsg::AddRound {
            round,
            previous_signature,
            signature,
        } => execute_add_round(deps, env, info, round, previous_signature, signature),
        ExecuteMsg::RegisterBot { moniker } => execute_register_bot(deps, env, info, moniker),
        ExecuteMsg::UpdateAllowlistBots { add, remove } => {
            execute_update_allowlist_bots(deps, info, add, remove)
        }
        ExecuteMsg::SetGatewayAddr { addr } => execute_set_gateway_addr(deps, env, addr),
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
        QueryMsg::Bot { address } => to_binary(&query_bot(deps, address)?)?,
        QueryMsg::Bots {} => to_binary(&query_bots(deps)?)?,
        QueryMsg::AllowList {} => to_binary(&query_allow_list(deps)?)?,
        QueryMsg::IsAllowListed { bot } => to_binary(&query_is_allow_listed(deps, bot)?)?,
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
    let prefix = SUBMISSIONS_ORDER.prefix(round);

    let submission_addresses: Vec<Addr> = prefix
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| -> StdResult<_> { Ok(item?.1) })
        .collect::<Result<_, _>>()?;
    let mut submissions: Vec<QueriedSubmission> = Vec::with_capacity(submission_addresses.len());
    for addr in submission_addresses {
        let stored = SUBMISSIONS.load(deps.storage, (round, &addr))?;
        submissions.push(QueriedSubmission::make(stored, addr));
    }
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

fn query_allow_list(deps: Deps) -> StdResult<AllowListResponse> {
    // No pagination here yet 🤷‍♂️
    let allowed = ALLOWLIST
        .range(deps.storage, None, None, Order::Ascending)
        .map(|result| {
            let (address, _) = result.unwrap();
            address.into()
        })
        .collect();
    Ok(AllowListResponse { allowed })
}

fn query_is_allow_listed(deps: Deps, bot: String) -> StdResult<IsAllowListedResponse> {
    let bot_addr = deps.api.addr_validate(&bot)?;
    let listed = ALLOWLIST.has(deps.storage, &bot_addr);
    Ok(IsAllowListedResponse { listed })
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
        },
    };
    BOTS.save(deps.storage, &info.sender, &bot)?;
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
    round: u64,
    previous_signature: HexBinary,
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

    // Initialise the incentive to 0
    let mut reward_points = Uint128::new(0);

    // Get the number of submission before this one.
    // The submissions are indexed 0-based, i.e. the number of elements is
    // the last index + 1 or 0 if no last index exists.
    let submissions_count = match SUBMISSIONS_ORDER
        .prefix(round)
        .keys(deps.storage, None, None, Order::Descending)
        .next()
    {
        Some(last_item) => last_item? + 1, // The ? handles the decoding to u32
        None => 0,
    };

    let randomness: HexBinary = derive_randomness(signature.as_slice()).into();
    // Check if we need to verify the submission  or we just compare it to the registered randomness from the first submission of this round
    let is_verifying_tx: bool;

    if submissions_count < NUMBER_OF_SUBMISSION_VERIFICATION_PER_ROUND {
        is_verifying_tx = true;
        // Check if the drand public key is valid
        let pk = g1_from_fixed_unchecked(DRAND_MAINNET_PUBKEY)
            .map_err(|_| ContractError::InvalidPubkey {})?;
        // Verify BLS
        if !verify(&pk, round, &previous_signature, &signature).unwrap_or(false) {
            return Err(ContractError::InvalidSignature {});
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

    // True if and only if bot has been registered before
    let mut is_registered = false;

    if let Some(mut bot) = BOTS.may_load(deps.storage, &info.sender)? {
        is_registered = true;
        bot.rounds_added += 1;
        BOTS.save(deps.storage, &info.sender, &bot)?;
    }
    let is_allowlisted = ALLOWLIST.has(deps.storage, &info.sender);

    SUBMISSIONS.save(
        deps.storage,
        submissions_key,
        &StoredSubmission {
            time: env.block.time,
        },
    )?;

    SUBMISSIONS_ORDER.save(deps.storage, (round, submissions_count), &info.sender)?;

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
                msg: to_binary(&NoisGatewayExecuteMsg::AddVerifiedRound {
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

    attributes.push(Attribute::new(ATTR_REWARD_POINTS, reward_points));

    // Pay the bot incentive
    // For now a bot needs to be registered, allowlisted and fast to  get incentives.
    // We can easily make unregistered bots eligible for incentives as well by changing
    // the following line

    let is_eligible = is_registered && is_allowlisted && !reward_points.is_zero(); // Allowed and registered bot that gathered contribution points get incentives

    if is_eligible {
        let desired_amount = reward_points * config.incentive_point_price;

        let contract_balance = deps
            .querier
            .query_balance(&env.contract.address, &config.incentive_denom)?
            .amount;

        // The amount we'll actually pay out
        let payout = if contract_balance >= desired_amount {
            Coin {
                amount: desired_amount,
                denom: config.incentive_denom,
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

/// In order not to fall in the chicken egg problem where you need
/// to instantiate two or more contracts that need to be aware of each other
/// in a context where the contract addresses generration is not known
/// in advance, we set the contract address at a later stage after the
/// instantation and make sure it is immutable once set
fn execute_set_gateway_addr(
    deps: DepsMut,
    _env: Env,
    addr: String,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    // ensure immutability
    if config.gateway.is_some() {
        return Err(ContractError::ContractAlreadySet {});
    }

    let nois_gateway = deps.api.addr_validate(&addr)?;
    config.gateway = Some(nois_gateway.clone());

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("nois-gateway-address", nois_gateway))
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::msg::ExecuteMsg;

    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{from_binary, Addr, Timestamp, Uint128};

    const TESTING_MANAGER: &str = "mnrg";
    const TESTING_MIN_ROUND: u64 = 72785;

    fn make_add_round_msg(round: u64) -> ExecuteMsg {
        match round {
            9 => ExecuteMsg::AddRound {
                // curl -sS https://drand.cloudflare.com/public/9
                round: 9,
                previous_signature: HexBinary::from_hex("b3ed3c540ef5c5407ea6dbf7407ca5899feeb54f66f7e700ee063db71f979a869d28efa9e10b5e6d3d24a838e8b6386a15b411946c12815d81f2c445ae4ee1a7732509f0842f327c4d20d82a1209f12dbdd56fd715cc4ed887b53c321b318cd7").unwrap(),
                signature: HexBinary::from_hex("99c37c83a0d7bb637f0e2f0c529aa5c8a37d0287535debe5dacd24e95b6e38f3394f7cb094bdf4908a192a3563276f951948f013414d927e0ba8c84466b4c9aea4de2a253dfec6eb5b323365dfd2d1cb98184f64c22c5293c8bfe7962d4eb0f5").unwrap(),
            },
            72785 => ExecuteMsg::AddRound {
                // curl -sS https://drand.cloudflare.com/public/72785
                round: 72785,
                previous_signature: HexBinary::from_hex("a609e19a03c2fcc559e8dae14900aaefe517cb55c840f6e69bc8e4f66c8d18e8a609685d9917efbfb0c37f058c2de88f13d297c7e19e0ab24813079efe57a182554ff054c7638153f9b26a60e7111f71a0ff63d9571704905d3ca6df0b031747").unwrap(),
                signature: HexBinary::from_hex("82f5d3d2de4db19d40a6980e8aa37842a0e55d1df06bd68bddc8d60002e8e959eb9cfa368b3c1b77d18f02a54fe047b80f0989315f83b12a74fd8679c4f12aae86eaf6ab5690b34f1fddd50ee3cc6f6cdf59e95526d5a5d82aaa84fa6f181e42").unwrap(),
            },
            72786 => ExecuteMsg::AddRound {
                // curl -sS https://drand.cloudflare.com/public/72786
                round: 72786,
                previous_signature: HexBinary::from_hex("82f5d3d2de4db19d40a6980e8aa37842a0e55d1df06bd68bddc8d60002e8e959eb9cfa368b3c1b77d18f02a54fe047b80f0989315f83b12a74fd8679c4f12aae86eaf6ab5690b34f1fddd50ee3cc6f6cdf59e95526d5a5d82aaa84fa6f181e42").unwrap(),
                signature: HexBinary::from_hex("85d64193239c6a2805b5953521c1e7c412d13f8b29df2dfc796b7dc8e1fd795b764362e49302956a350f9385f68b68d8085fda08c2bd0528984a413db52860b408c72d1210609de3a342259d4c08f86ee729a2dbeb140908270849fd7d0dec40").unwrap(),
            },
            72787 => ExecuteMsg::AddRound {
                // curl -sS https://drand.cloudflare.com/public/72787
                round: 72787,
                previous_signature: HexBinary::from_hex("85d64193239c6a2805b5953521c1e7c412d13f8b29df2dfc796b7dc8e1fd795b764362e49302956a350f9385f68b68d8085fda08c2bd0528984a413db52860b408c72d1210609de3a342259d4c08f86ee729a2dbeb140908270849fd7d0dec40").unwrap(),
                signature: HexBinary::from_hex("8ceee95d523f54a752807f4705ce0f89e69911dd3dce330a337b9409905a881a2f879d48fce499bfeeb3b12e7f83ab7d09b42f31fa729af4c19adfe150075b2f3fe99c8fbcd7b0b5f0bb91ac8ad8715bfe52e3fb12314fddb76d4e42461f6ea4").unwrap(),
            },
            2183668 => ExecuteMsg::AddRound {
                // curl -sS https://drand.cloudflare.com/public/2183668
                round: 2183668,
                previous_signature: HexBinary::from_hex("b0272269d87be8f146a0dc4f882b03add1e0f98ee7c55ee674107c231cfa7d2e40d9c88dd6e72f2f52d1abe14766b2c40dd392eec82d678a4c925c6937717246e8ae96d54d8ea70f85f8282cf14c56e5b547b7ee82df4ff61f3523a0eefcdf41").unwrap(),
                signature: HexBinary::from_hex("b06969214b8a7c8d705c4c5e00262626d95e30f8583dc21670508d6d4751ae95ddf675e76feabe1ee5f4000dd21f09d009bb2b57da6eedd10418e83c303c2d5845914175ffe13601574d039a7593c3521eaa98e43be927b4a00d423388501f05").unwrap(),
            },
            2183669 => ExecuteMsg::AddRound {
                // curl -sS https://drand.cloudflare.com/public/2183669
                round: 2183669,
                previous_signature: HexBinary::from_hex("b06969214b8a7c8d705c4c5e00262626d95e30f8583dc21670508d6d4751ae95ddf675e76feabe1ee5f4000dd21f09d009bb2b57da6eedd10418e83c303c2d5845914175ffe13601574d039a7593c3521eaa98e43be927b4a00d423388501f05").unwrap(),
                signature: HexBinary::from_hex("990538b0f0ca3b934f53eb41d7a4ba24f3b3800abfc06275eb843df75a53257c2dbfb8f6618bb72874a79303429db13e038e6619c08726e8bbb3ae58ebb31e08d2aed921e4246fdef984285eb679c6b443f24bd04f78659bd4230e654db4200d").unwrap(),
            },
            2183670 => ExecuteMsg::AddRound {
                // curl -sS https://drand.cloudflare.com/public/2183670
                round: 2183670,
                previous_signature: HexBinary::from_hex("990538b0f0ca3b934f53eb41d7a4ba24f3b3800abfc06275eb843df75a53257c2dbfb8f6618bb72874a79303429db13e038e6619c08726e8bbb3ae58ebb31e08d2aed921e4246fdef984285eb679c6b443f24bd04f78659bd4230e654db4200d").unwrap(),
                signature: HexBinary::from_hex("a63dcbd669534b049a86198ee98f1b68c24aac50de411d11f2a8a98414f9312cd04027810417d0fa60461c0533d604630ada568ef83af93ce05c1620c8bee1491092c11e5c7d9bb679b5b8de61bbb48e092164366ae6f799c082ddab691d1d78").unwrap(),
            },
            2183671 => ExecuteMsg::AddRound {
                // curl -sS https://drand.cloudflare.com/public/2183671
                round: 2183671,
                previous_signature: HexBinary::from_hex("a63dcbd669534b049a86198ee98f1b68c24aac50de411d11f2a8a98414f9312cd04027810417d0fa60461c0533d604630ada568ef83af93ce05c1620c8bee1491092c11e5c7d9bb679b5b8de61bbb48e092164366ae6f799c082ddab691d1d78").unwrap(),
                signature: HexBinary::from_hex("b449f94098616029baea233fa8b64851cf9de2b230a7c5a2181c3abdc9e92806ae9020a5d9dcdbb707b6f1754480954b00a80b594cb35b51944167d2b20cc3b3cac6da7023c6a6bf867c6c3844768794edcaae292394316603797d669f62691a").unwrap(),
            },
            _ => panic!("Test round {round} not set"),
        }
    }

    /// Adds round 72785, 72786, 72787
    fn add_test_rounds(mut deps: DepsMut, bot_addr: &str) {
        let msg = make_add_round_msg(72785);
        execute(deps.branch(), mock_env(), mock_info(bot_addr, &[]), msg).unwrap();
        let msg = make_add_round_msg(72786);
        execute(deps.branch(), mock_env(), mock_info(bot_addr, &[]), msg).unwrap();
        let msg = make_add_round_msg(72787);
        execute(deps.branch(), mock_env(), mock_info(bot_addr, &[]), msg).unwrap();
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
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
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
    fn register_bot(deps: DepsMut, info: MessageInfo) {
        let register_bot_msg = ExecuteMsg::RegisterBot {
            moniker: "Best Bot".to_string(),
        };
        execute(deps, mock_env(), info, register_bot_msg).unwrap();
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

        let info = mock_info("anyone", &[]);
        register_bot(deps.as_mut(), info.to_owned());

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
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
        assert_eq!(min_round, TESTING_MIN_ROUND);

        let msg = make_add_round_msg(9);
        let err = execute(deps.as_mut(), mock_env(), mock_info("anyone", &[]), msg).unwrap_err();
        assert!(matches!(
            err,
            ContractError::RoundTooLow {
                round: 9,
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

        let msg = ExecuteMsg::AddRound {
            // curl -sS https://drand.cloudflare.com/public/72785
            round: 72785,
            previous_signature: HexBinary::from_hex("a609e19a03c2fcc559e8dae14900aaefe517cb55c840f6e69bc8e4f66c8d18e8a609685d9917efbfb0c37f058c2de88f13d297c7e19e0ab24813079efe57a182554ff054c7638153f9b26a60e7111f71a0ff63d9571704905d3ca6df0b031747").unwrap(),
            signature: HexBinary::from_hex("82f5d3d2de4db19d40a6980e8aa37842a0e55d1df06bd68bddc8d60002e8e959eb9cfa368b3c1b77d18f02a54fe047b80f0989315f83b12a74fd8679c4f12aae86eaf6ab5690b34f1fddd50ee3cc6f6cdf59e95526d5a5d82aaa84fa6f181e42").unwrap(),
        };
        let info = mock_info("unregistered_bot", &[]);
        let response = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let randomness = first_attr(response.attributes, "randomness").unwrap();
        assert_eq!(
            randomness,
            "8b676484b5fb1f37f9ec5c413d7d29883504e5b669f604a1ce68b3388e9ae3d9"
        );
        assert_eq!(response.messages.len(), 0);
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

        let IsAllowListedResponse { listed } = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::IsAllowListed {
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

        let IsAllowListedResponse { listed } = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::IsAllowListed {
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

        let IsAllowListedResponse { listed } = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::IsAllowListed {
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

        let IsAllowListedResponse { listed } = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::IsAllowListed {
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
                assert_eq!(msg, "Invalid input: human address too short")
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
                assert_eq!(msg, "Invalid input: human address too short")
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

        let info = mock_info("registered_bot", &[]);
        register_bot(deps.as_mut(), info);
        // add bot to allowlist
        let info = mock_info(TESTING_MANAGER, &[]);
        let msg = ExecuteMsg::UpdateAllowlistBots {
            add: vec!["registered_bot".to_string()],
            remove: vec![],
        };
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg = make_add_round_msg(72785);
        let info = mock_info("registered_bot", &[]);
        let response = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(response.messages.len(), 0);
        let attrs = response.attributes;
        assert_eq!(
            first_attr(&attrs, "randomness").unwrap(),
            "8b676484b5fb1f37f9ec5c413d7d29883504e5b669f604a1ce68b3388e9ae3d9"
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

        let bot1 = "registered_bot1";
        let bot2 = "registered_bot2";
        let bot3 = "registered_bot3";
        let bot4 = "registered_bot4";
        let bot5 = "registered_bot5";
        let bot6 = "registered_bot6";
        let bot7 = "registered_bot7";

        register_bot(deps.as_mut(), mock_info(bot1, &[]));
        register_bot(deps.as_mut(), mock_info(bot2, &[]));
        register_bot(deps.as_mut(), mock_info(bot3, &[]));
        register_bot(deps.as_mut(), mock_info(bot4, &[]));
        register_bot(deps.as_mut(), mock_info(bot5, &[]));
        register_bot(deps.as_mut(), mock_info(bot6, &[]));
        register_bot(deps.as_mut(), mock_info(bot7, &[]));

        // add bots to allowlist
        let msg = ExecuteMsg::UpdateAllowlistBots {
            add: vec![
                "registered_bot1".to_string(),
                "registered_bot2".to_string(),
                "registered_bot3".to_string(),
                "registered_bot4".to_string(),
                "registered_bot5".to_string(),
                "registered_bot6".to_string(),
                "registered_bot7".to_string(),
            ],
            remove: vec![],
        };
        let info = mock_info(TESTING_MANAGER, &[]);
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Same msg for all submissions
        let msg = make_add_round_msg(72785);

        // 1st
        let info = mock_info(bot1, &[]);
        let response = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();
        assert_eq!(response.messages.len(), 1);

        // 2nd
        let info = mock_info(bot2, &[]);
        let response = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();
        assert_eq!(response.messages.len(), 1);

        // 3rd
        let info = mock_info(bot3, &[]);
        let response = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();
        assert_eq!(response.messages.len(), 1);

        // 4th
        let info = mock_info(bot4, &[]);
        let response = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();
        assert_eq!(response.messages.len(), 1);

        // 5th
        let info = mock_info(bot5, &[]);
        let response = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();
        assert_eq!(response.messages.len(), 1);

        // 6th
        let info = mock_info(bot6, &[]);
        let response = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();
        assert_eq!(response.messages.len(), 1);

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

        let msg = make_add_round_msg(72785);
        let info = mock_info("unregistered_bot", &[]);
        let response = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let randomness = first_attr(response.attributes, "randomness").unwrap();
        assert_eq!(
            randomness,
            "8b676484b5fb1f37f9ec5c413d7d29883504e5b669f604a1ce68b3388e9ae3d9"
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

        let info = mock_info("anyone", &[]);
        register_bot(deps.as_mut(), info.to_owned());
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
        let msg = InstantiateMsg {
            manager: TESTING_MANAGER.to_string(),
            min_round: TESTING_MIN_ROUND,
            incentive_point_price: Uint128::new(20_000),
            incentive_denom: "unois".to_string(),
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg = ExecuteMsg::AddRound {
            // curl -sS https://drand.cloudflare.com/public/72785
            round: 79999, // wrong round
            previous_signature: hex::decode("a609e19a03c2fcc559e8dae14900aaefe517cb55c840f6e69bc8e4f66c8d18e8a609685d9917efbfb0c37f058c2de88f13d297c7e19e0ab24813079efe57a182554ff054c7638153f9b26a60e7111f71a0ff63d9571704905d3ca6df0b031747").unwrap().into(),
            signature: hex::decode("82f5d3d2de4db19d40a6980e8aa37842a0e55d1df06bd68bddc8d60002e8e959eb9cfa368b3c1b77d18f02a54fe047b80f0989315f83b12a74fd8679c4f12aae86eaf6ab5690b34f1fddd50ee3cc6f6cdf59e95526d5a5d82aaa84fa6f181e42").unwrap().into(),
        };
        let result = execute(deps.as_mut(), mock_env(), mock_info("anon", &[]), msg);
        match result.unwrap_err() {
            ContractError::InvalidSignature {} => {}
            err => panic!("Unexpected error: {:?}", err),
        };

        let msg = ExecuteMsg::AddRound {
            // curl -sS https://drand.cloudflare.com/public/72785
            round: 72785,
            // wrong previous_signature
            previous_signature: hex::decode("cccccccccccccccc59e8dae14900aaefe517cb55c840f6e69bc8e4f66c8d18e8a609685d9917efbfb0c37f058c2de88f13d297c7e19e0ab24813079efe57a182554ff054c7638153f9b26a60e7111f71a0ff63d9571704905d3ca6df0b031747").unwrap().into(),
            signature: hex::decode("82f5d3d2de4db19d40a6980e8aa37842a0e55d1df06bd68bddc8d60002e8e959eb9cfa368b3c1b77d18f02a54fe047b80f0989315f83b12a74fd8679c4f12aae86eaf6ab5690b34f1fddd50ee3cc6f6cdf59e95526d5a5d82aaa84fa6f181e42").unwrap().into(),
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

        let info = mock_info("creator", &[]);
        register_bot(deps.as_mut(), info.to_owned());
        let msg = InstantiateMsg {
            manager: TESTING_MANAGER.to_string(),
            min_round: TESTING_MIN_ROUND,
            incentive_point_price: Uint128::new(20_000),
            incentive_denom: "unois".to_string(),
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg = make_add_round_msg(72785);

        // Execute 1
        let info = mock_info("anyone", &[]);
        register_bot(deps.as_mut(), info.to_owned());
        let response = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();
        let randomness = first_attr(response.attributes, "randomness").unwrap();
        assert_eq!(
            randomness,
            "8b676484b5fb1f37f9ec5c413d7d29883504e5b669f604a1ce68b3388e9ae3d9"
        );

        // Execute 2
        let info = mock_info("someone else", &[]);
        register_bot(deps.as_mut(), info.to_owned());
        let response = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        let randomness = first_attr(response.attributes, "randomness").unwrap();
        assert_eq!(
            randomness,
            "8b676484b5fb1f37f9ec5c413d7d29883504e5b669f604a1ce68b3388e9ae3d9"
        );
    }

    #[test]
    fn add_round_fails_when_same_bot_submits_multiple_times() {
        let mut deps = mock_dependencies();

        let info = mock_info("creator", &[]);
        register_bot(deps.as_mut(), info.to_owned());
        let msg = InstantiateMsg {
            manager: TESTING_MANAGER.to_string(),
            min_round: TESTING_MIN_ROUND,
            incentive_point_price: Uint128::new(20_000),
            incentive_denom: "unois".to_string(),
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg = make_add_round_msg(72785);

        // Execute A1
        let info = mock_info("bot_alice", &[]);
        register_bot(deps.as_mut(), info.to_owned());
        execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();
        // Execute B1
        let info = mock_info("bot_bob", &[]);
        register_bot(deps.as_mut(), info.to_owned());
        execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();

        // Execute A2
        let info = mock_info("bot_alice", &[]);
        register_bot(deps.as_mut(), info.to_owned());
        let err = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
        assert!(matches!(err, ContractError::SubmissionExists));
        // Execute B2
        let info = mock_info("bot_alice", &[]);
        register_bot(deps.as_mut(), info.to_owned());
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
        let BotResponse { bot } = from_binary(
            &query(
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
            }
        );

        // re-register

        let info = mock_info(&bot_addr, &[]);
        let register_bot_msg = ExecuteMsg::RegisterBot {
            moniker: "Another nickname".to_string(),
        };
        execute(deps.as_mut(), mock_env(), info, register_bot_msg).unwrap();
        let BotResponse { bot } = from_binary(
            &query(
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

        let info = mock_info("anyone", &[]);
        register_bot(deps.as_mut(), info);
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
        register_bot(deps.as_mut(), info.to_owned());
        let msg = InstantiateMsg {
            manager: TESTING_MANAGER.to_string(),
            min_round: TESTING_MIN_ROUND,
            incentive_point_price: Uint128::new(20_000),
            incentive_denom: "unois".to_string(),
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let info = mock_info("anyone", &[]);
        register_bot(deps.as_mut(), info);
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
        register_bot(deps.as_mut(), info.to_owned());
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

        let info = mock_info(bot1, &[]);
        register_bot(deps.as_mut(), info);
        add_test_rounds(deps.as_mut(), bot1);

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
            [QueriedSubmission {
                bot: Addr::unchecked(bot1),
                time: Timestamp::from_nanos(1571797419879305533),
            }]
        );

        add_test_rounds(deps.as_mut(), bot2);

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
                QueriedSubmission {
                    bot: Addr::unchecked(bot1),
                    time: Timestamp::from_nanos(1571797419879305533),
                },
                QueriedSubmission {
                    bot: Addr::unchecked(bot2),
                    time: Timestamp::from_nanos(1571797419879305533),
                },
            ]
        );

        add_test_rounds(deps.as_mut(), bot3);

        // Three submissions
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
                QueriedSubmission {
                    bot: Addr::unchecked(bot1),
                    time: Timestamp::from_nanos(1571797419879305533),
                },
                QueriedSubmission {
                    bot: Addr::unchecked(bot2),
                    time: Timestamp::from_nanos(1571797419879305533),
                },
                QueriedSubmission {
                    bot: Addr::unchecked(bot3),
                    time: Timestamp::from_nanos(1571797419879305533),
                },
            ]
        );
    }

    #[test]
    fn query_allow_list_works() {
        let mut deps = mock_dependencies();

        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {
            manager: TESTING_MANAGER.to_string(),
            min_round: TESTING_MIN_ROUND,
            incentive_point_price: Uint128::new(20_000),
            incentive_denom: "unois".to_string(),
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let AllowListResponse { allowed } =
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::AllowList {}).unwrap())
                .unwrap();
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

        let AllowListResponse { allowed } =
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::AllowList {}).unwrap())
                .unwrap();
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

        let AllowListResponse { allowed } =
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::AllowList {}).unwrap())
                .unwrap();
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
    fn is_query_allow_listed_works() {
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
        let IsAllowListedResponse { listed } = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::IsAllowListed {
                    bot: "bot_b".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert!(listed);

        // bot_a is not listed
        let IsAllowListedResponse { listed } = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::IsAllowListed {
                    bot: "bot_a".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert!(!listed);
    }
}
