use cosmwasm_std::{
    entry_point, to_binary, CheckedFromRatioError, Decimal, Deps, DepsMut, Env, MessageInfo, Order,
    QueryResponse, Response, StdResult, WasmMsg,
};
use nois_proxy::{Data, NoisCallbackMsg};
use nois_toolbox::{random_decimal, sub_randomness};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{NOIS_PROXY, RESULTS};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    NOIS_PROXY.save(deps.storage, &msg.nois_proxy)?;
    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("nois_proxy", msg.nois_proxy))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::EstimatePi { job_id } => execute_estimate_pi(deps, env, info, job_id),
        ExecuteMsg::Receive(NoisCallbackMsg {
            id: callback_id,
            randomness,
        }) => execute_receive(deps, env, info, callback_id, randomness),
    }
}

pub fn execute_estimate_pi(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    job_id: String,
) -> Result<Response, ContractError> {
    let nois_proxy = NOIS_PROXY.load(deps.storage)?;

    let res = Response::new().add_message(WasmMsg::Execute {
        contract_addr: nois_proxy,
        msg: to_binary(&nois_proxy::ExecuteMsg::GetNextRandomness {
            callback_id: Some(job_id),
        })?,
        funds: vec![],
    });
    Ok(res)
}

pub fn execute_receive(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    callback_id: String,
    randomness: Data,
) -> Result<Response, ContractError> {
    let randomness: [u8; 32] = randomness
        .to_array()
        .map_err(|_| ContractError::InvalidRandomness)?;

    let mut provider = sub_randomness(randomness);

    const ROUNDS: u32 = 10_000;
    let mut inside = 0u32;

    for _round in 0..ROUNDS {
        let x = random_decimal(provider.provide());
        let y = random_decimal(provider.provide());
        // sqrt calculation can be skipped because a² < 1 if and only if a < 1
        let in_circle = (x * x) + (y * y) < Decimal::one();
        if in_circle {
            inside += 1;
        }
    }

    let in_circle_ratio = match Decimal::checked_from_ratio(inside, ROUNDS) {
        Ok(val) => val,
        Err(CheckedFromRatioError::Overflow) => panic!("Input value too low to exceed range"),
        Err(CheckedFromRatioError::DivideByZero) => panic!("Number of rounds must not ne zero"),
    };
    let four = Decimal::from_atomics(4u32, 0).unwrap();
    let estimated_pi = in_circle_ratio * four;

    RESULTS.save(deps.storage, &callback_id, &estimated_pi)?;

    Ok(Response::default())
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<QueryResponse> {
    match msg {
        QueryMsg::Results {} => to_binary(&query_results(deps)?),
        QueryMsg::Result { job_id } => to_binary(&query_result(deps, job_id)?),
    }
}

fn query_results(deps: Deps) -> StdResult<Vec<String>> {
    let out: Vec<String> = RESULTS
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| item.map(|(id, value)| format!("{id}:{value}")))
        .collect::<StdResult<_>>()?;
    Ok(out)
}

fn query_result(deps: Deps, job_id: String) -> StdResult<Option<Decimal>> {
    let result = RESULTS.may_load(deps.storage, &job_id)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    const CREATOR: &str = "creator";

    #[test]
    fn instantiate_works() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            nois_proxy: "address123".to_string(),
        };
        let info = mock_info(CREATOR, &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }
}
