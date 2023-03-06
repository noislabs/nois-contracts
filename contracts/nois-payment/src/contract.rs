use cosmwasm_std::{
    entry_point, to_binary, BankMsg, Coin, CosmosMsg, DepsMut, Env, MessageInfo, Response, Uint128,
    WasmMsg,
};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, NoisSinkExecuteMsg};
use crate::state::{COMMUNITY_POOL, SINK};

/// Constant defining the denom of the Coin to be burnt
const PAYMENT_DENOM: &str = "unois";

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let nois_sink_addr = deps
        .api
        .addr_validate(&msg.nois_sink)
        .map_err(|_| ContractError::InvalidAddress)?;
    SINK.save(deps.storage, &nois_sink_addr)?;
    let nois_com_pool_addr = deps
        .api
        .addr_validate(&msg.nois_com_pool_addr)
        .map_err(|_| ContractError::InvalidAddress)?;
    COMMUNITY_POOL.save(deps.storage, &nois_com_pool_addr)?;
    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("nois_sink", msg.nois_sink)
        .add_attribute("nois_community_pool", msg.nois_com_pool_addr))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Pay {
            burn,
            community_pool,
            relayer,
        } => execute_pay(deps, info, env, burn, community_pool, relayer),
    }
}

// #[entry_point]
// pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<QueryResponse> {
//     let response = match msg {};
//     Ok(response)
// }

fn execute_pay(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    burn: Uint128,
    community_pool: Uint128,
    relayer: (String, Uint128),
) -> Result<Response, ContractError> {
    let funds = info.funds;

    // Check there are no funds. Not a payable Msg
    if !funds.is_empty() {
        return Err(ContractError::DontSendFunds);
    }
    // Check relayer addr is valid
    deps.api
        .addr_validate(relayer.0.as_str())
        .map_err(|_| ContractError::InvalidAddress)?;

    // Burn
    let mut out_msgs: Vec<CosmosMsg> = vec![WasmMsg::Execute {
        contract_addr: SINK.load(deps.storage).unwrap().to_string(),
        msg: to_binary(&NoisSinkExecuteMsg::Burn {})?,
        funds: vec![Coin::new(burn.into(), PAYMENT_DENOM)],
    }
    .into()];

    // Send to relayer
    out_msgs.push(
        BankMsg::Send {
            to_address: relayer.0.to_owned(),
            amount: vec![Coin::new(relayer.1.into(), PAYMENT_DENOM)],
        }
        .into(),
    );

    // Send to community pool
    out_msgs.push(
        BankMsg::Send {
            to_address: COMMUNITY_POOL.load(deps.storage).unwrap().to_string(),
            amount: vec![Coin::new(community_pool.into(), PAYMENT_DENOM)],
        }
        .into(),
    );

    Ok(Response::new()
        .add_messages(out_msgs)
        .add_attribute("burnt_amount", burn)
        .add_attribute("relayer_incentive", relayer.1)
        .add_attribute("relayer_address", relayer.0)
        .add_attribute("sent_to_community_pool", community_pool))
}

#[cfg(test)]
mod tests {}
