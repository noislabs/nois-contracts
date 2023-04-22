use cosmwasm_std::{
    ensure_eq, entry_point, to_binary, Addr, Attribute, BankMsg, Coin, Deps, DepsMut, Env,
    HexBinary, MessageInfo, QueryResponse, Response, StdResult, Timestamp, Uint128, WasmMsg,
};
use nois::{NoisCallback, ProxyExecuteMsg};
use sha2::{Digest, Sha256};

use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, IsClaimedResponse, IsLuckyResponse,
    MerkleRootResponse, QueryMsg,
};
use crate::state::{Config, RandomnessParams, CLAIM, CONFIG, MERKLE_ROOT, NOIS_RANDOMNESS};

// The airdrop Denom, probably gonna be some IBCed Nois something like IBC/hashhashhashhashhashhash
const AIRDROP_DENOM: &str = "unois";
const AIRDROP_ODDS: u8 = 3;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let manager = deps.api.addr_validate(&msg.manager)?;
    let nois_proxy = deps
        .api
        .addr_validate(&msg.nois_proxy)
        .map_err(|_| ContractError::InvalidProxyAddress)?;

    NOIS_RANDOMNESS.save(
        deps.storage,
        &RandomnessParams {
            nois_proxy,
            nois_randomness: None,
            requested: false,
        },
    )?;

    let config = Config { manager };
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
        ExecuteMsg::UpdateConfig { manager } => execute_update_config(deps, env, info, manager),
        ExecuteMsg::RegisterMerkleRoot { merkle_root } => {
            execute_register_merkle_root(deps, env, info, merkle_root)
        }
        //RandDrop should be called by the manager with a future timestamp
        ExecuteMsg::RandDrop {
            random_beacon_after,
        } => execute_rand_drop(deps, env, info, random_beacon_after),
        //NoisReceive should be called by the proxy contract. The proxy is forwarding the randomness from the nois chain to this contract.
        ExecuteMsg::NoisReceive { callback } => execute_receive(deps, env, info, callback),
        ExecuteMsg::Claim { amount, proof } => execute_claim(deps, env, info, amount, proof),
        ExecuteMsg::WithdawAll { address } => execute_withdraw_all(deps, env, info, address),
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<QueryResponse> {
    let response = match msg {
        QueryMsg::IsLucky { address } => to_binary(&query_is_lucky(deps, address)?)?,
        QueryMsg::Config {} => to_binary(&query_config(deps)?)?,
        QueryMsg::MerkleRoot {} => to_binary(&query_merkle_root(deps)?)?,
        QueryMsg::IsClaimed { address } => to_binary(&query_is_claimed(deps, address)?)?,
    };
    Ok(response)
}

fn execute_update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    manager: Option<String>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    // check the calling address is the authorised multisig
    ensure_eq!(info.sender, config.manager, ContractError::Unauthorized);

    let manager = match manager {
        Some(ma) => deps.api.addr_validate(&ma)?,
        None => config.manager,
    };

    CONFIG.save(deps.storage, &Config { manager })?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

// This function will call the proxy and ask for the randomness round
pub fn execute_rand_drop(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    random_beacon_after: Timestamp,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    // check the calling address is the authorised multisig
    ensure_eq!(info.sender, config.manager, ContractError::Unauthorized);

    // For full transparency make sure the merkle root has been registered before beacon request
    if MERKLE_ROOT.may_load(deps.storage)?.is_none() {
        return Err(ContractError::MerkleRootAbsent);
    }

    let RandomnessParams {
        nois_proxy,
        nois_randomness,
        requested,
    } = NOIS_RANDOMNESS.load(deps.storage).unwrap();
    // Prevents requesting randomness twice.
    if requested {
        return Err(ContractError::ImmutableRandomness);
    }
    NOIS_RANDOMNESS.save(
        deps.storage,
        &RandomnessParams {
            nois_proxy: nois_proxy.clone(),
            nois_randomness,
            requested: true,
        },
    )?;

    let response = Response::new().add_message(WasmMsg::Execute {
        contract_addr: nois_proxy.into(),
        // GetRandomnessAfter requests the randomness from the proxy after a specific timestamp
        // The job id is needed to know what randomness we are referring to upon reception in the callback.
        // In this example we only need 1 random number so this can be hardcoded to "airdrop"
        msg: to_binary(&ProxyExecuteMsg::GetRandomnessAfter {
            after: random_beacon_after,
            job_id: "airdrop".to_string(),
        })?,
        // We pay here the proxy contract with whatever the manager sends. The manager needs to check in advance the proxy prices.
        funds: info.funds, // Just pass on all funds we got
    });
    Ok(response)
}

pub fn execute_receive(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    callback: NoisCallback,
) -> Result<Response, ContractError> {
    let RandomnessParams {
        nois_proxy,
        nois_randomness,
        requested,
    } = NOIS_RANDOMNESS.load(deps.storage).unwrap();

    // callback should only be allowed to be called by the proxy contract
    // otherwise anyone can cut the randomness workflow and cheat the randomness by sending the randomness directly to this contract
    ensure_eq!(info.sender, nois_proxy, ContractError::UnauthorizedReceive);
    let randomness: [u8; 32] = callback
        .randomness
        .to_array()
        .map_err(|_| ContractError::InvalidRandomness)?;
    // Make sure the randomness does not exist yet

    match nois_randomness {
        None => NOIS_RANDOMNESS.save(
            deps.storage,
            &RandomnessParams {
                nois_proxy,
                nois_randomness: Some(randomness),
                requested,
            },
        ),
        Some(_randomness) => return Err(ContractError::ImmutableRandomness),
    }?;

    Ok(Response::default())
}

fn execute_withdraw_all(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    // check the calling address is the authorised multisig
    ensure_eq!(info.sender, config.manager, ContractError::Unauthorized);

    let amount = deps
        .querier
        .query_balance(env.contract.address, AIRDROP_DENOM)?;
    let msg = BankMsg::Send {
        to_address: address,
        amount: vec![amount],
    };
    let res = Response::new()
        .add_message(msg)
        .add_attribute("action", "withdraw_all");
    Ok(res)
}

fn is_randomly_eligible(addr: &Addr, randomness: [u8; 32]) -> bool {
    let mut hasher = Sha256::new();
    hasher.update(addr.as_bytes());
    let sender_hash: [u8; 32] = hasher.finalize().into();
    // Concatenate the randomness and sender hash values
    let combined = [randomness, sender_hash].concat();
    // Hash the combined value using SHA256 to generate a random number between 1 and 3
    let mut hasher = Sha256::new();
    hasher.update(&combined);
    let hash = hasher.finalize();
    let outcome = hash[0] % AIRDROP_ODDS;

    // returns true if the address is eligible
    outcome == 0
}

fn execute_register_merkle_root(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    merkle_root: HexBinary,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let current_merkle_root = MERKLE_ROOT.may_load(deps.storage)?;

    // check the calling address is the authorised multisig
    ensure_eq!(info.sender, config.manager, ContractError::Unauthorized);

    if current_merkle_root.is_some() {
        return Err(ContractError::MerkleImmutable {});
    }

    if merkle_root.len() != 32 {
        return Err(ContractError::WrongLength {});
    }

    MERKLE_ROOT.save(deps.storage, &merkle_root)?;

    Ok(Response::new().add_attributes(vec![
        Attribute::new("action", "register_merkle_root"),
        Attribute::new("merkle_root", merkle_root.to_string()),
    ]))
}

fn execute_claim(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
    proof: Vec<HexBinary>,
) -> Result<Response, ContractError> {
    // verify not claimed
    let claimed = CLAIM.may_load(deps.storage, info.sender.clone())?;
    if claimed.is_some() {
        return Err(ContractError::Claimed {});
    }
    let merkle_root = MERKLE_ROOT.load(deps.storage)?;

    // "nois1blabla...chksum4500000" -> hash
    let user_input = format!("{}{}", info.sender, amount);
    let hash = sha2::Sha256::digest(user_input.as_bytes()).into();

    // hash all the way up the merkle tree until reaching the top root.
    let hash = proof
        .into_iter()
        .try_fold(hash, |hash, p| -> Result<_, ContractError> {
            let proof_buf: [u8; 32] = p.to_array()?;
            let mut hashes = [hash, proof_buf];
            hashes.sort_unstable();
            Ok(sha2::Sha256::digest(hashes.concat()).into())
        })?;

    // Check the overall cumulated proof hashes along the merkle tree ended up having the same hash as the registered Merkle root
    if merkle_root != hash {
        return Err(ContractError::VerificationFailed {});
    }

    // Check that the sender is lucky enough to be randomly eligible for the randdrop
    let nois_randomness = NOIS_RANDOMNESS.load(deps.storage).unwrap().nois_randomness;

    match nois_randomness {
        Some(randomness) => match is_randomly_eligible(&info.sender, randomness) {
            true => Ok(()),
            false => Err(ContractError::NotRandomlyEligible {}),
        },
        None => Err(ContractError::RandomnessUnavailable {}),
    }?;

    // Update claim
    CLAIM.save(deps.storage, info.sender.clone(), &true)?;

    let res = Response::new()
        .add_message(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![Coin {
                amount: amount * Uint128::new(AIRDROP_ODDS as u128),
                denom: AIRDROP_DENOM.to_string(),
            }],
        })
        .add_attributes(vec![
            Attribute::new("action", "claim"),
            Attribute::new("address", info.sender),
            Attribute::new("amount", amount),
        ]);
    Ok(res)
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        manager: config.manager.to_string(),
    })
}
fn query_is_lucky(deps: Deps, address: String) -> StdResult<IsLuckyResponse> {
    let address = deps.api.addr_validate(address.as_str())?;
    // Check if the address is lucky to be randomly selected for the randdrop
    let nois_randomness = NOIS_RANDOMNESS.load(deps.storage).unwrap().nois_randomness;

    let is_lucky = nois_randomness.map(|randomness| is_randomly_eligible(&address, randomness));
    Ok(IsLuckyResponse { is_lucky })
}

fn query_merkle_root(deps: Deps) -> StdResult<MerkleRootResponse> {
    let merkle_root = MERKLE_ROOT.load(deps.storage)?;
    let resp = MerkleRootResponse { merkle_root };

    Ok(resp)
}

fn query_is_claimed(deps: Deps, address: String) -> StdResult<IsClaimedResponse> {
    let is_claimed = CLAIM
        .may_load(deps.storage, deps.api.addr_validate(&address)?)?
        .unwrap_or(false);
    let resp = IsClaimedResponse { is_claimed };

    Ok(resp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{
        mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
    };
    use cosmwasm_std::{from_binary, from_slice, Empty, HexBinary, OwnedDeps, SubMsg};
    use serde::Deserialize;

    const CREATOR: &str = "creator";
    const PROXY_ADDRESS: &str = "the proxy of choice";
    const MANAGER: &str = "manager1";

    fn instantiate_contract() -> OwnedDeps<MockStorage, MockApi, MockQuerier, Empty> {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            manager: MANAGER.to_string(),
            nois_proxy: PROXY_ADDRESS.to_string(),
        };
        let info = mock_info(CREATOR, &[]);
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        deps
    }

    #[test]
    fn proper_instantiation() {
        let deps = instantiate_contract();
        let env = mock_env();

        // it worked, let's query the state
        let res = query(deps.as_ref(), env, QueryMsg::Config {}).unwrap();
        let config: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!(MANAGER, config.manager.as_str());
    }
    #[test]
    fn instantiate_fails_for_invalid_proxy_address() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            manager: MANAGER.to_string(),
            nois_proxy: "".to_string(),
        };
        let info = mock_info("CREATOR", &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(res, ContractError::InvalidProxyAddress);
    }

    #[test]
    fn update_config() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            manager: MANAGER.to_string(),
            nois_proxy: "nois_proxy".to_string(),
        };

        let env = mock_env();
        let info = mock_info(MANAGER, &[]);
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        // update manager
        let env = mock_env();
        let info = mock_info(MANAGER, &[]);
        let msg = ExecuteMsg::UpdateConfig {
            manager: Some("manager2".to_string()),
        };

        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(deps.as_ref(), env, QueryMsg::Config {}).unwrap();
        let config: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!("manager2", config.manager.as_str());

        // Unauthorized err
        let env = mock_env();
        let info = mock_info(MANAGER, &[]);
        let msg = ExecuteMsg::UpdateConfig { manager: None };

        let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(res, ContractError::Unauthorized {});
    }

    #[test]
    fn register_merkle_root() {
        let mut deps = instantiate_contract();

        // register new merkle root
        let env = mock_env();
        let info = mock_info(MANAGER, &[]);
        let msg = ExecuteMsg::RegisterMerkleRoot {
            merkle_root: HexBinary::from_hex("634de21cde").unwrap(),
        };
        let err = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
        assert_eq!(err, ContractError::WrongLength {});
        let msg = ExecuteMsg::RegisterMerkleRoot {
            merkle_root: HexBinary::from_hex(
                "634de21cde1044f41d90373733b0f0fb1c1c71f9652b905cdf159e73c4cf0d37",
            )
            .unwrap(),
        };

        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                Attribute::new("action", "register_merkle_root"),
                Attribute::new(
                    "merkle_root",
                    "634de21cde1044f41d90373733b0f0fb1c1c71f9652b905cdf159e73c4cf0d37"
                )
            ]
        );

        let res = query(deps.as_ref(), env, QueryMsg::MerkleRoot {}).unwrap();
        let merkle_root: MerkleRootResponse = from_binary(&res).unwrap();
        assert_eq!(
            HexBinary::from_hex("634de21cde1044f41d90373733b0f0fb1c1c71f9652b905cdf159e73c4cf0d37")
                .unwrap(),
            merkle_root.merkle_root
        );
        // registering a new merkle root should fail
        let env = mock_env();
        let info = mock_info(MANAGER, &[]);
        let msg = ExecuteMsg::RegisterMerkleRoot {
            merkle_root: HexBinary::from_hex(
                "634de21cde1044f41d90373733b0f0fb1c1c71f9652b905cdf159e73c4cf0d37",
            )
            .unwrap(),
        };
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(err, ContractError::MerkleImmutable {});
    }

    const TEST_DATA: &[u8] =
        include_bytes!("../../../tests/airdrop/nois_testnet_005_test_data.json");
    const TEST_DATA_LIST: &[u8] =
        include_bytes!("../../../tests/airdrop/nois_testnet_005_list.json");

    #[derive(Deserialize, Debug)]
    struct Encoded {
        account: String,
        amount: Uint128,
        root: HexBinary,
        proofs: Vec<HexBinary>,
    }
    #[derive(Deserialize, Debug)]
    struct Account {
        address: Addr,
        //amount: u64,
    }
    #[derive(Deserialize, Debug)]
    struct AirdropList {
        airdrop_list: Vec<Account>,
    }

    #[test]
    fn execute_rand_drop_works() {
        let mut deps = instantiate_contract();

        let msg = ExecuteMsg::RandDrop {
            random_beacon_after: Timestamp::from_seconds(11111111),
        };
        let info = mock_info("guest", &[]);
        // Only manager should be able to request the randomness
        let err = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});

        let info = mock_info(MANAGER, &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
        assert_eq!(err, ContractError::MerkleRootAbsent {});

        let info = mock_info(MANAGER, &[]);
        let msg_merkle = ExecuteMsg::RegisterMerkleRoot {
            merkle_root: HexBinary::from_hex(
                "634de21cde1044f41d90373733b0f0fb1c1c71f9652b905cdf159e73c4cf0d37",
            )
            .unwrap(),
        };
        execute(deps.as_mut(), mock_env(), info, msg_merkle).unwrap();

        let info = mock_info(MANAGER, &[]);
        execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

        // Cannot request randomness more than once
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, ContractError::ImmutableRandomness {});
    }

    #[test]
    fn execute_receive_works() {
        let mut deps = instantiate_contract();

        let msg = ExecuteMsg::NoisReceive {
            callback: NoisCallback {
                job_id: "123".to_string(),
                randomness: HexBinary::from_hex(
                    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                )
                .unwrap(),
            },
        };
        let info = mock_info("some_random_account", &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
        // Only proxy should call this entrypoint
        assert_eq!(err, ContractError::UnauthorizedReceive {});
        let info = mock_info(PROXY_ADDRESS, &[]);
        execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();
        let info = mock_info(PROXY_ADDRESS, &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        // Proxy should cannot call this entrypoint if there's already randomness in state
        assert_eq!(err, ContractError::ImmutableRandomness {});
    }

    #[test]
    fn claim() {
        // Run test 1
        let mut deps = instantiate_contract();
        let test_data: Encoded = from_slice(TEST_DATA).unwrap();

        let env = mock_env();
        let info = mock_info(MANAGER, &[]);
        let msg = ExecuteMsg::RegisterMerkleRoot {
            merkle_root: test_data.root,
        };
        let _res = execute(deps.as_mut(), env, info, msg).unwrap();

        // Receive randomness
        let msg = ExecuteMsg::NoisReceive {
            callback: NoisCallback {
                job_id: "123".to_string(),
                randomness: HexBinary::from_hex(
                    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa124",
                )
                .unwrap(),
            },
        };
        let info = mock_info(PROXY_ADDRESS, &[]);
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg = ExecuteMsg::Claim {
            amount: test_data.amount,
            proof: test_data.proofs,
        };

        let env = mock_env();
        let info = mock_info(test_data.account.as_str(), &[]);
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
        let expected = SubMsg::new(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![Coin {
                amount: Uint128::new(13500000), // 4500000*3
                denom: AIRDROP_DENOM.to_string(),
            }],
        });
        assert_eq!(res.messages, vec![expected]);

        assert_eq!(
            res.attributes,
            vec![
                Attribute::new("action", "claim"),
                Attribute::new("address", test_data.account.clone()),
                Attribute::new("amount", test_data.amount)
            ]
        );

        assert!(
            from_binary::<IsClaimedResponse>(
                &query(
                    deps.as_ref(),
                    env.clone(),
                    QueryMsg::IsClaimed {
                        address: test_data.account
                    }
                )
                .unwrap()
            )
            .unwrap()
            .is_claimed
        );
        // Try and claim again
        let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(res, ContractError::Claimed {});

        // Stop aridrop and Widhdraw funds
        let env = mock_env();
        let info = mock_info("random_person_who_hates_airdrops", &[]);
        let msg = ExecuteMsg::WithdawAll {
            address: "some-address".to_string(),
        };
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});

        let env = mock_env();
        let info = mock_info(MANAGER, &[]);
        let msg = ExecuteMsg::WithdawAll {
            address: "withdraw_address".to_string(),
        };
        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        let expected = SubMsg::new(BankMsg::Send {
            to_address: "withdraw_address".to_string(),
            amount: vec![Coin {
                amount: Uint128::new(0),
                denom: AIRDROP_DENOM.to_string(),
            }],
        });
        assert_eq!(res.messages, vec![expected]);
    }

    #[test]
    fn randomness_elgibility_distribution_is_correct() {
        let mut deps = instantiate_contract();
        let test_data_json: Encoded = from_slice(TEST_DATA).unwrap();
        let merkle_root = test_data_json.root;

        let test_data: AirdropList = from_slice(TEST_DATA_LIST).unwrap();

        let env = mock_env();
        let info = mock_info(MANAGER, &[]);
        let msg = ExecuteMsg::RegisterMerkleRoot { merkle_root };
        execute(deps.as_mut(), env, info, msg).unwrap();

        // Receive randomness
        let msg = ExecuteMsg::NoisReceive {
            callback: NoisCallback {
                job_id: "123".to_string(),
                randomness: HexBinary::from_hex(
                    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa124",
                )
                .unwrap(),
            },
        };
        let info = mock_info(PROXY_ADDRESS, &[]);
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let mut num_lucky = 0;
        for addr in &test_data.airdrop_list {
            let response: IsLuckyResponse = from_binary(
                &query(
                    deps.as_ref(),
                    mock_env(),
                    QueryMsg::IsLucky {
                        address: addr.address.to_string(),
                    },
                )
                .unwrap(),
            )
            .unwrap();

            if response.is_lucky.unwrap_or_default() {
                num_lucky += 1;
            }
        }
        // Normally we should tolerate some difference here but I guess with thislist we were lucky to get exactly the 3rd 17/51
        assert_eq!(
            num_lucky,
            test_data.airdrop_list.len() / AIRDROP_ODDS as usize
        );
    }
}
