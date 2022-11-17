use cosmwasm_std::{
    entry_point,  Deps, DepsMut, Env, Event, MessageInfo,
    Order, QueryResponse, Response, StdError, StdResult, Timestamp, Uint128, to_binary, ensure_eq, BankMsg, Coin, CosmosMsg,
};

use crate::error::ContractError;
use crate::msg::{ ExecuteMsg, InstantiateMsg, QueryMsg, ConfigResponse};
use crate::state::{
     Config, CONFIG,
};



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
        ExecuteMsg::IncentiviseBot {addr} => execute_incentivise_bot(deps, env, info, addr),
        ExecuteMsg::Stake { addr, amount} => execute_stake(deps, env, info, addr, amount),
        ExecuteMsg::Unbond { addr, amount } => execute_unbond(deps, env, info, addr, amount),
        ExecuteMsg::Redelegate { src_addr,dest_addr, amount } => execute_redelegate(deps, env, info, src_addr, dest_addr, amount),
        ExecuteMsg::ClaimRewards {} => execute_claim_rewards(deps, env, info),
        ExecuteMsg::SetNoisOracleContractAddr { addr } =>execute_set_nois_oracle_contract_addr(deps, env, info, addr),
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
    _env: Env,
    info: MessageInfo,
    addr: String,
) -> Result<Response, ContractError> {

    let config = CONFIG.load(deps.storage)?;
    if config.nois_oracle_contract_addr.is_none(){
        return Err(ContractError::NoisOracleContractAddressUnset);
    }
    let nois_oracle_contract=config.nois_oracle_contract_addr.unwrap();
    
    ensure_eq!(info.sender, nois_oracle_contract, ContractError::Unauthorized);
    let mut out_msgs = Vec::<CosmosMsg>::new();
    out_msgs.push(
    BankMsg::Send {
        to_address: addr, //Not sure if here we can exract the drand_bot addr by info.sender. Is info.sender here the nois-oracle or the drand bot?
        amount: vec![Coin::new(config.incentive_amount.into() , config.incentive_denom)],
    }.into());

    Ok(Response::new()
        .add_messages(out_msgs))
}

fn execute_stake(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    addr: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    
    
    Ok(Response::default())
}

fn execute_unbond(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    addr: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    
    Ok(Response::default())
}

fn execute_redelegate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    src_addr: String,
    dest_addr: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    
    Ok(Response::default())
}

fn execute_claim_rewards(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    
    Ok(Response::default())
}

fn execute_set_nois_oracle_contract_addr(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    addr: String,
)-> Result<Response, ContractError> {
    let mut config: Config= CONFIG.load(deps.storage)?;
    if config.nois_oracle_contract_addr.is_some(){
        return Err(ContractError::ContractAlreadySet {});
    }
    let nois_contract = deps
        .api
        .addr_validate(&addr)
        .map_err(|_| ContractError::InvalidAddress)?;
    config.nois_oracle_contract_addr=Some(nois_contract);

    CONFIG.save(deps.storage, &config);
        
    Ok(Response::default())
}


fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config)
}