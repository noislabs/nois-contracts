use cosmwasm_std::{
    ensure_eq, entry_point, to_binary, BankMsg, Coin, Deps, DepsMut, DistributionMsg, Env,
    MessageInfo, QueryResponse, Response, StakingMsg, StdResult, Uint128,
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
    let manager = deps.api.addr_validate(&msg.manager)?;
    let config = Config {
        manager,
        drand: None,
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
        ExecuteMsg::SendFundsToDrand { funds } => execute_send_funds_to_drand(deps, env, funds),
        ExecuteMsg::Delegate { addr, amount } => execute_delegate(deps, info, addr, amount),
        ExecuteMsg::Undelegate { addr, amount } => execute_undelegate(deps, info, addr, amount),
        ExecuteMsg::Redelegate {
            src_addr,
            dest_addr,
            amount,
        } => execute_redelegate(deps, info, src_addr, dest_addr, amount),
        ExecuteMsg::ClaimRewards { addr } => execute_claim_rewards(addr),
        ExecuteMsg::SetDrandAddr { addr } => execute_set_drand_addr(deps, env, addr),
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
fn execute_send_funds_to_drand(
    deps: DepsMut,
    _env: Env,
    funds: Coin,
) -> Result<Response, ContractError> {
    let Some(nois_drand_contract) = CONFIG.load(deps.storage)?.drand else {
        return Err(ContractError::NoisDrandAddressUnset);
    };

    Ok(Response::new()
        .add_attribute("nois-icecube-sent-amount", funds.to_string())
        .add_message(BankMsg::Send {
            to_address: nois_drand_contract.to_string(),
            amount: vec![funds],
        }))
}

/// This function will delegate staked coins
/// to one validator with the addr address
fn execute_delegate(
    deps: DepsMut,
    info: MessageInfo,
    addr: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // check the calling address is the authorised multisig
    ensure_eq!(
        info.sender,
        CONFIG.load(deps.storage)?.manager,
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
    deps: DepsMut,
    info: MessageInfo,
    addr: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // check the calling address is the authorised multisig
    ensure_eq!(
        info.sender,
        CONFIG.load(deps.storage)?.manager,
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
    deps: DepsMut,
    info: MessageInfo,
    src_addr: String,
    dest_addr: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // check the calling address is the authorised multisig
    ensure_eq!(
        info.sender,
        CONFIG.load(deps.storage)?.manager,
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
fn execute_set_drand_addr(
    deps: DepsMut,
    _env: Env,
    addr: String,
) -> Result<Response, ContractError> {
    let mut config: Config = CONFIG.load(deps.storage)?;

    // ensure immutability
    if config.drand.is_some() {
        return Err(ContractError::ContractAlreadySet {});
    }

    let nois_drand_address = deps.api.addr_validate(&addr)?;
    config.drand = Some(nois_drand_address.clone());

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("nois-drand-address", nois_drand_address))
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
    const MANAGER: &str = "the_manager_addr";

    #[test]
    fn instantiate_works() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            manager: MANAGER.to_string(),
        };
        let info = mock_info(CREATOR, &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        let config: ConfigResponse =
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
        assert_eq!(
            config,
            ConfigResponse {
                manager: Addr::unchecked(MANAGER),
                drand: None,
            }
        );
    }

    #[test]
    fn only_manager_can_delegate_undelegate_redelegate() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            manager: MANAGER.to_string(),
        };
        let info = mock_info(CREATOR, &[]);
        let _result = instantiate(deps.as_mut(), mock_env(), info, msg);

        // check manager operations work

        // delegate for manager works
        let info = mock_info(MANAGER, &[]);
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

        // undelegate for manager works
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

        // redelegate for manager works
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

        // check non-manager operations are unothorized

        // delegate for non-manager unauthorized
        let info = mock_info("not_manager", &[]);
        let msg = ExecuteMsg::Delegate {
            addr: "validator".to_string(),
            amount: Uint128::new(1_000),
        };
        let err = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized));

        // undelegate for non-manager unauthorized
        let msg = ExecuteMsg::Undelegate {
            addr: "validator".to_string(),
            amount: Uint128::new(1_000),
        };
        let err = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized));

        // redelegate for non-manager unauthorized
        let msg = ExecuteMsg::Redelegate {
            src_addr: "src_validator".to_string(),
            dest_addr: "dest_validator".to_string(),
            amount: Uint128::new(1_000),
        };
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized));
    }
}
