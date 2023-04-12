use cosmwasm_std::{
    entry_point, to_binary, Attribute, BankMsg, Coin, Deps, DepsMut, Env, MessageInfo,
    QueryResponse, Response, StdResult, Uint128,
};
use sha2::Digest;

use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, IsClaimedResponse, MerkleRootResponse, QueryMsg,
};
use crate::state::{Config, CLAIM, CONFIG, MERKLE_ROOT};

// The staking, unbonding, redelegating, claim denom. It can be the same as the incentive denom
const AIRDROP_DENOM: &str = "unois";

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let withdrawal_address = deps.api.addr_validate(&msg.withdrawal_address)?;
    let manager = msg
        .manager
        .map_or(Ok(info.sender), |o| deps.api.addr_validate(&o))?;

    let config = Config {
        manager: Some(manager),
        withdrawal_address,
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
        ExecuteMsg::UpdateConfig { new_manager } => {
            execute_update_config(deps, env, info, new_manager)
        }
        ExecuteMsg::RegisterMerkleRoot { merkle_root } => {
            execute_register_merkle_root(deps, env, info, merkle_root)
        }
        ExecuteMsg::Claim { amount, proof } => execute_claim(deps, env, info, amount, proof),
        ExecuteMsg::WithdawAll {} => execute_withdraw_all(deps, env, info),
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<QueryResponse> {
    let response = match msg {
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
    new_manager: Option<String>,
) -> Result<Response, ContractError> {
    // authorize manager
    let cfg = CONFIG.load(deps.storage)?;
    let manager = cfg.manager.ok_or(ContractError::Unauthorized {})?;
    if info.sender != manager {
        return Err(ContractError::Unauthorized {});
    }

    // if manager some validated to addr, otherwise set to none
    let mut tmp_manager = None;
    if let Some(addr) = new_manager {
        tmp_manager = Some(deps.api.addr_validate(&addr)?)
    }

    CONFIG.update(deps.storage, |mut exists| -> StdResult<_> {
        exists.manager = tmp_manager;
        Ok(exists)
    })?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

fn execute_withdraw_all(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // check the calling address is the authorised addr

    // if manager set validate, otherwise unauthorized
    let manager = config.manager.ok_or(ContractError::Unauthorized {})?;
    if info.sender != manager {
        return Err(ContractError::Unauthorized {});
    }

    let amount = deps
        .querier
        .query_balance(env.contract.address, AIRDROP_DENOM)?;
    let msg = BankMsg::Send {
        to_address: config.withdrawal_address.into(),
        amount: vec![amount],
    };
    let res = Response::new()
        .add_message(msg)
        .add_attribute("action", "withdraw_all");
    Ok(res)
}

fn execute_register_merkle_root(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    merkle_root: String,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;
    let current_merkle_root = MERKLE_ROOT.may_load(deps.storage)?;

    // if manager set validate, otherwise unauthorized
    let manager = cfg.manager.ok_or(ContractError::Unauthorized {})?;
    if info.sender != manager {
        return Err(ContractError::Unauthorized {});
    }

    if current_merkle_root.is_some() {
        return Err(ContractError::MerkleImmutable {});
    }

    // check merkle root length
    let mut root_buf: [u8; 32] = [0; 32];

    hex::decode_to_slice(merkle_root.clone(), &mut root_buf)?;

    MERKLE_ROOT.save(deps.storage, &merkle_root)?;

    Ok(Response::new().add_attributes(vec![
        Attribute::new("action", "register_merkle_root"),
        Attribute::new("merkle_root", merkle_root),
    ]))
}

fn execute_claim(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
    proof: Vec<String>,
) -> Result<Response, ContractError> {
    // verify not claimed
    let claimed = CLAIM.may_load(deps.storage, info.sender.clone())?;
    if claimed.is_some() {
        return Err(ContractError::Claimed {});
    }
    let merkle_root = MERKLE_ROOT.load(deps.storage)?;

    let user_input = format!("{}{}", info.sender, amount);
    let hash = sha2::Sha256::digest(user_input.as_bytes())
        .as_slice()
        .try_into()
        .map_err(|_| ContractError::WrongLength {})?;

    let hash = proof.into_iter().try_fold(hash, |hash, p| {
        let mut proof_buf = [0; 32];
        hex::decode_to_slice(p, &mut proof_buf)?;
        let mut hashes = [hash, proof_buf];
        hashes.sort_unstable();
        sha2::Sha256::digest(hashes.concat())
            .as_slice()
            .try_into()
            .map_err(|_| ContractError::WrongLength {})
    })?;

    let mut root_buf: [u8; 32] = [0; 32];
    hex::decode_to_slice(merkle_root, &mut root_buf)?;
    if root_buf != hash {
        return Err(ContractError::VerificationFailed {});
    }

    // Update claim
    CLAIM.save(deps.storage, info.sender.clone(), &true)?;

    let res = Response::new()
        .add_message(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![Coin {
                amount,
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
    let cfg = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        manager: cfg.manager.map(|o| o.to_string()),
    })
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
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{from_binary, from_slice, SubMsg};
    use serde::Deserialize;

    #[test]
    fn proper_instantiation() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            manager: Some("manager1".to_string()),
            withdrawal_address: "withdraw_address".to_string(),
        };

        let env = mock_env();
        let info = mock_info("addr-1", &[]);

        // we can just call .unwrap() to assert this was a success
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        // it worked, let's query the state
        let res = query(deps.as_ref(), env, QueryMsg::Config {}).unwrap();
        let config: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!("manager1", config.manager.unwrap().as_str());
    }

    #[test]
    fn update_config() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            manager: None,
            withdrawal_address: "withdraw_address".to_string(),
        };

        let env = mock_env();
        let info = mock_info("manager1", &[]);
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        // update manager
        let env = mock_env();
        let info = mock_info("manager1", &[]);
        let msg = ExecuteMsg::UpdateConfig {
            new_manager: Some("manager2".to_string()),
        };

        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(deps.as_ref(), env, QueryMsg::Config {}).unwrap();
        let config: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!("manager2", config.manager.unwrap().as_str());

        // Unauthorized err
        let env = mock_env();
        let info = mock_info("manager1", &[]);
        let msg = ExecuteMsg::UpdateConfig { new_manager: None };

        let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(res, ContractError::Unauthorized {});
    }

    #[test]
    fn register_merkle_root() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            manager: Some("manager1".to_string()),
            withdrawal_address: "withdraw_address".to_string(),
        };

        let env = mock_env();
        let info = mock_info("addr-1", &[]);
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        // register new merkle root
        let env = mock_env();
        let info = mock_info("manager1", &[]);
        let msg = ExecuteMsg::RegisterMerkleRoot {
            merkle_root: "634de21cde1044f41d90373733b0f0fb1c1c71f9652b905cdf159e73c4cf0d37"
                .to_string(),
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
            "634de21cde1044f41d90373733b0f0fb1c1c71f9652b905cdf159e73c4cf0d37".to_string(),
            merkle_root.merkle_root
        );
        // registering a new merkle root should fail
        let env = mock_env();
        let info = mock_info("manager1", &[]);
        let msg = ExecuteMsg::RegisterMerkleRoot {
            merkle_root: "634de21cde1044f41d90373733b0f0fb1c1c71f9652b905cdf159e73c4cf0d37"
                .to_string(),
        };
        let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(res, ContractError::MerkleImmutable {});
    }

    const TEST_DATA: &[u8] =
        include_bytes!("../../../tests/airdrop/nois_testnet_005_test_data.json");

    #[derive(Deserialize, Debug)]
    struct Encoded {
        account: String,
        amount: Uint128,
        root: String,
        proofs: Vec<String>,
    }

    #[test]
    fn claim() {
        // Run test 1
        let mut deps = mock_dependencies();
        let test_data: Encoded = from_slice(TEST_DATA).unwrap();

        let msg = InstantiateMsg {
            manager: Some("manager1".to_string()),
            withdrawal_address: "withdraw_address".to_string(),
        };

        let env = mock_env();
        let info = mock_info("addr-1", &[]);
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let env = mock_env();
        let info = mock_info("manager1", &[]);
        let msg = ExecuteMsg::RegisterMerkleRoot {
            merkle_root: test_data.root,
        };
        let _res = execute(deps.as_mut(), env, info, msg).unwrap();

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
                amount: Uint128::new(4500000),
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
        let msg = ExecuteMsg::WithdawAll {};
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});

        let env = mock_env();
        let info = mock_info("manager1", &[]);
        let msg = ExecuteMsg::WithdawAll {};
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
    fn manager_freeze() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            manager: Some("manager1".to_string()),
            withdrawal_address: "withdraw_address".to_string(),
        };

        let env = mock_env();
        let info = mock_info("addr-1", &[]);
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        // can register merkle root
        let env = mock_env();
        let info = mock_info("manager1", &[]);
        let msg = ExecuteMsg::RegisterMerkleRoot {
            merkle_root: "5d4f48f147cb6cb742b376dce5626b2a036f69faec10cd73631c791780e150fc"
                .to_string(),
        };
        let _res = execute(deps.as_mut(), env, info, msg).unwrap();

        // can update manager
        let env = mock_env();
        let info = mock_info("manager1", &[]);
        let msg = ExecuteMsg::UpdateConfig {
            new_manager: Some("manager2".to_string()),
        };

        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // freeze contract
        let env = mock_env();
        let info = mock_info("manager2", &[]);
        let msg = ExecuteMsg::UpdateConfig { new_manager: None };

        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // cannot register new drop
        let env = mock_env();
        let info = mock_info("manager2", &[]);
        let msg = ExecuteMsg::RegisterMerkleRoot {
            merkle_root: "ebaa83c7eaf7467c378d2f37b5e46752d904d2d17acd380b24b02e3b398b3e5a"
                .to_string(),
        };
        let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(res, ContractError::Unauthorized {});

        // cannot update config
        let env = mock_env();
        let info = mock_info("manager2", &[]);
        let msg = ExecuteMsg::RegisterMerkleRoot {
            merkle_root: "ebaa83c7eaf7467c378d2f37b5e46752d904d2d17acd380b24b02e3b398b3e5a"
                .to_string(),
        };
        let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(res, ContractError::Unauthorized {});
    }
}
