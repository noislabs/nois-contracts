use cosmwasm_std::{
    ensure_eq, entry_point, to_binary, BankMsg, Coin, Deps, DepsMut, DistributionMsg, Env,
    MessageInfo, QueryResponse, Response, StakingMsg, StdResult, Uint128,
};

use crate::error::ContractError;
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Config, CONFIG};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let config = Config {
        admin_addr: msg.admin_addr,
        incentive_amount: msg.incentive_amount,
        incentive_denom: msg.incentive_denom,
        staking_denom: msg.staking_denom,
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
        ExecuteMsg::IncentiviseBot { addr } => execute_incentivise_bot(deps, info, addr),
        ExecuteMsg::Stake { addr, amount } => execute_stake(deps, addr, amount),
        ExecuteMsg::Unbond { addr, amount } => execute_unbond(deps, addr, amount),
        ExecuteMsg::Redelegate {
            src_addr,
            dest_addr,
            amount,
        } => execute_redelegate(deps, src_addr, dest_addr, amount),
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

fn execute_incentivise_bot(
    deps: DepsMut,
    info: MessageInfo,
    addr: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if config.nois_oracle_contract_addr.is_none() {
        return Err(ContractError::NoisOracleContractAddressUnset);
    }
    let nois_oracle_contract = config.nois_oracle_contract_addr.unwrap();

    ensure_eq!(
        info.sender,
        nois_oracle_contract,
        ContractError::Unauthorized
    );

    Ok(Response::new().add_messages(vec![BankMsg::Send {
        to_address: addr, //Not sure if here we can exract the drand_bot addr by info.sender. Is info.sender here the nois-oracle or the drand bot?
        amount: vec![Coin::new(
            config.incentive_amount.into(),
            config.incentive_denom,
        )],
    }]))
}

fn execute_stake(deps: DepsMut, addr: String, amount: Uint128) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage).unwrap();

    Ok(Response::new().add_messages(vec![StakingMsg::Delegate {
        validator: addr,
        amount: Coin {
            denom: config.staking_denom,
            amount,
        },
    }]))
}

fn execute_unbond(deps: DepsMut, addr: String, amount: Uint128) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage).unwrap();

    Ok(Response::new().add_messages(vec![StakingMsg::Undelegate {
        validator: addr,
        amount: Coin {
            denom: config.staking_denom,
            amount,
        },
    }]))
}

fn execute_redelegate(
    deps: DepsMut,
    src_addr: String,
    dest_addr: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage).unwrap();

    Ok(Response::new().add_messages(vec![StakingMsg::Redelegate {
        src_validator: src_addr,
        dst_validator: dest_addr,
        amount: Coin {
            denom: config.staking_denom,
            amount,
        },
    }]))
}

fn execute_claim_rewards(addr: String) -> Result<Response, ContractError> {
    Ok(
        Response::new().add_messages(vec![DistributionMsg::WithdrawDelegatorReward {
            validator: addr,
        }]),
    )
}

fn execute_set_nois_oracle_contract_addr(
    deps: DepsMut,
    _env: Env,
    addr: String,
) -> Result<Response, ContractError> {
    let mut config: Config = CONFIG.load(deps.storage)?;
    if config.nois_oracle_contract_addr.is_some() {
        return Err(ContractError::ContractAlreadySet {});
    }
    let nois_contract = deps
        .api
        .addr_validate(&addr)
        .map_err(|_| ContractError::InvalidAddress)?;
    config.nois_oracle_contract_addr = Some(nois_contract);

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default())
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
        Uint128,
    };

    const CREATOR: &str = "creator";

    #[test]
    fn instantiate_works() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            admin_addr: "admin".to_string(),
            incentive_denom: "unois".to_string(),
            staking_denom: "unois".to_string(),
            incentive_amount: Uint128::new(1_000_000),
        };
        let info = mock_info(CREATOR, &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        let config: ConfigResponse =
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
        assert_eq!(
            config,
            ConfigResponse {
                incentive_amount: Uint128::new(1_000_000),
                incentive_denom: "unois".to_string(),
                admin_addr: "admin".to_string(),
                staking_denom: "unois".to_string(),
                nois_oracle_contract_addr: None,
            }
        );
    }

    //#[test]
    //fn staking_works() {
    //    let mut deps = mock_dependencies();
    //    let msg = InstantiateMsg {
    //        admin_addr: "admin".to_string(),
    //        incentive_denom:"unois".to_string(),
    //        staking_denom:"unois".to_string(),
    //        incentive_amount:Uint128::new(1_000_000),
    //
    //    };
    //    let  env = mock_env();
    //    let info = mock_info(CREATOR, &[]);
    //    instantiate(deps.as_mut(), env.to_owned(), info.clone(), msg).unwrap();
    //    let addr="validator_addr".to_string();
    //    let amount=Uint128::new(100);
    //    let msg = ExecuteMsg::Stake { addr: addr.to_owned(), amount } ;
    //    execute(deps.as_mut(),env.to_owned() , info, msg).unwrap();
    //    let msg= StakingQuery::Delegation { delegator: env.to_owned().contract.address.into_string(), validator: addr.to_string() };
    //
    //}
}
