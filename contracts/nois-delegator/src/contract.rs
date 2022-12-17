use cosmwasm_std::{
    ensure_eq, entry_point, to_binary, Attribute, BankMsg, Coin, Deps, DepsMut, DistributionMsg,
    Env, MessageInfo, QueryResponse, Response, StakingMsg, StdResult, Uint128,
};

use crate::error::ContractError;
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Config, CONFIG};

// The staking, unbonding, redelegating, claim denom. It can be the same as the incentive denom
const STAKING_DENOM: &str = "unois";

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let admin_addr = deps
        .api
        .addr_validate(&msg.admin_addr)
        .map_err(|_| ContractError::InvalidAddress)
        .unwrap();
    let config = Config {
        admin_addr,
        nois_oracle_contract_addr: None,
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
        ExecuteMsg::SendFundsToOracle { amount } => {
            execute_send_funds_to_oracle(deps.as_ref(), env, amount)
        }
        ExecuteMsg::Delegate { addr, amount } => {
            execute_delegate(deps.as_ref(), info, addr, amount)
        }
        ExecuteMsg::Undelegate { addr, amount } => {
            execute_undelegate(deps.as_ref(), info, addr, amount)
        }
        ExecuteMsg::Redelegate {
            src_addr,
            dest_addr,
            amount,
        } => execute_redelegate(deps.as_ref(), info, src_addr, dest_addr, amount),
        ExecuteMsg::ClaimRewards { addr } => execute_claim_rewards(addr),
        ExecuteMsg::SetNoisOracleContractAddr { addr } => {
            execute_set_nois_oracle_contract_addr(deps, env, addr)
        }
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<QueryResponse> {
    let response = match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?)?,
    };
    Ok(response)
}

/// This function will send incentive coins to incentivize a bot
/// The bot is normally incentivised for bringing randomness on chain
/// But could also be incentivised for executing extra things
/// like callback jobs
fn execute_send_funds_to_oracle(
    deps: Deps,
    env: Env,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let Some(nois_oracle_contract) = CONFIG.load(deps.storage).unwrap().nois_oracle_contract_addr else {
        return Err(ContractError::NoisOracleContractAddressUnset);
    };

    // Check that this contract has the requested amount
    let contract_balance = deps
        .querier
        .query_balance(&env.contract.address, STAKING_DENOM)?
        .amount;
    if contract_balance < amount {
        return Err(ContractError::InsufficientBalance);
    }

    Ok(Response::new()
        .add_attribute("nois-delegator-sent-amount", amount)
        .add_message(BankMsg::Send {
            to_address: nois_oracle_contract.to_string(),
            amount: vec![Coin::new(amount.into(), STAKING_DENOM)],
        }))
}

/// This function will delegate staked coins
/// to one validator with the addr address
fn execute_delegate(
    deps: Deps,
    info: MessageInfo,
    addr: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // check the calling address is the authorised multisig
    ensure_eq!(
        info.sender,
        CONFIG.load(deps.storage)?.admin_addr,
        ContractError::Unauthorized
    );

    Ok(Response::new().add_message(StakingMsg::Delegate {
        validator: addr,
        amount: Coin {
            denom: STAKING_DENOM.to_string(),
            amount,
        },
    }))
}
/// This function will undelegate staked coins
/// from one validator with the addr address
fn execute_undelegate(
    deps: Deps,
    info: MessageInfo,
    addr: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // check the calling address is the authorised multisig
    ensure_eq!(
        info.sender,
        CONFIG.load(deps.storage)?.admin_addr,
        ContractError::Unauthorized
    );

    Ok(Response::new().add_message(StakingMsg::Undelegate {
        validator: addr,
        amount: Coin {
            denom: STAKING_DENOM.to_string(),
            amount,
        },
    }))
}
/// This function will make this contract move the bonded stakes
/// from one validator (src_addr) to another validator (dest_addr)
fn execute_redelegate(
    deps: Deps,
    info: MessageInfo,
    src_addr: String,
    dest_addr: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // check the calling address is the authorised multisig
    ensure_eq!(
        info.sender,
        CONFIG.load(deps.storage)?.admin_addr,
        ContractError::Unauthorized
    );

    Ok(Response::new().add_message(StakingMsg::Redelegate {
        src_validator: src_addr,
        dst_validator: dest_addr,
        amount: Coin {
            denom: STAKING_DENOM.to_string(),
            amount,
        },
    }))
}

/// This function will make this contract claim the staking rewards accumulated
/// by staking to a specific validator.
/// This function is permissionless. Anyone can claim rewards for the contract
fn execute_claim_rewards(addr: String) -> Result<Response, ContractError> {
    Ok(Response::new().add_message(DistributionMsg::WithdrawDelegatorReward { validator: addr }))
}

/// In order not to fall in the chicken egg problem where you need
/// to instantiate two or more contracts that need to be aware of each other
/// in a context where the contract addresses generration is not known
/// in advance, we set the contract address at a later stage after the
/// instantation and make sure it is immutable once set
fn execute_set_nois_oracle_contract_addr(
    deps: DepsMut,
    _env: Env,
    addr: String,
) -> Result<Response, ContractError> {
    let mut config: Config = CONFIG.load(deps.storage)?;

    // ensure immutability
    if config.nois_oracle_contract_addr.is_some() {
        return Err(ContractError::ContractAlreadySet {});
    }

    let nois_contract = deps
        .api
        .addr_validate(&addr)
        .map_err(|_| ContractError::InvalidAddress)?;
    config.nois_oracle_contract_addr = Some(nois_contract.clone());

    CONFIG.save(deps.storage, &config)?;
    let attributes = vec![Attribute::new(
        "nois-oracle-address",
        nois_contract.to_string(),
    )];

    Ok(Response::new().add_attributes(attributes))
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config)
}

#[cfg(test)]
mod tests {

    use super::*;
    use cosmwasm_std::{
        from_binary,
        testing::{mock_dependencies, mock_env, mock_info},
        Addr, CosmosMsg, Uint128,
    };

    const CREATOR: &str = "creator";

    #[test]
    fn instantiate_works() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            admin_addr: "admin".to_string(),
        };
        let info = mock_info(CREATOR, &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        let config: ConfigResponse =
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
        assert_eq!(
            config,
            ConfigResponse {
                admin_addr: Addr::unchecked("admin"),
                nois_oracle_contract_addr: None,
            }
        );
    }

    #[test]
    fn only_admin_can_delegate_undelegate_redelegate() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            admin_addr: "admin".to_string(),
        };
        let info = mock_info(CREATOR, &[]);
        let _result = instantiate(deps.as_mut(), mock_env(), info, msg);

        // check admin operations work

        // delegate for admin works
        let info = mock_info("admin", &[]);
        let msg = ExecuteMsg::Delegate {
            addr: "validator".to_string(),
            amount: Uint128::new(1_000),
        };

        let response = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        assert_eq!(
            response.messages[0].msg,
            CosmosMsg::Staking(StakingMsg::Delegate {
                validator: "validator".to_string(),
                amount: Coin {
                    amount: Uint128::new(1_000),
                    denom: "unois".to_string()
                },
            })
        );

        // undelegate for admin works
        let msg = ExecuteMsg::Undelegate {
            addr: "validator".to_string(),
            amount: Uint128::new(1_000),
        };

        let response = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        assert_eq!(
            response.messages[0].msg,
            CosmosMsg::Staking(StakingMsg::Undelegate {
                validator: "validator".to_string(),
                amount: Coin {
                    amount: Uint128::new(1_000),
                    denom: "unois".to_string()
                },
            })
        );

        // redelegate for admin works
        let msg = ExecuteMsg::Redelegate {
            src_addr: "src_validator".to_string(),
            dest_addr: "dest_validator".to_string(),
            amount: Uint128::new(1_000),
        };

        let response = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(
            response.messages[0].msg,
            CosmosMsg::Staking(StakingMsg::Redelegate {
                src_validator: "src_validator".to_string(),
                dst_validator: "dest_validator".to_string(),
                amount: Coin {
                    amount: Uint128::new(1_000),
                    denom: "unois".to_string()
                },
            })
        );

        // check non admin operations are unothorized

        // delegate for non admin unothorized
        let info = mock_info("not_admin", &[]);
        let msg = ExecuteMsg::Delegate {
            addr: "validator".to_string(),
            amount: Uint128::new(1_000),
        };
        let err = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized));

        // undelegate for non admin unothorized
        let msg = ExecuteMsg::Undelegate {
            addr: "validator".to_string(),
            amount: Uint128::new(1_000),
        };
        let err = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized));

        // redelegate for non admin unothorized
        let msg = ExecuteMsg::Redelegate {
            src_addr: "src_validator".to_string(),
            dest_addr: "dest_validator".to_string(),
            amount: Uint128::new(1_000),
        };
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized));
    }
}
