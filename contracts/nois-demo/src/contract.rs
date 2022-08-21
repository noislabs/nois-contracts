use cosmwasm_std::{
    entry_point, to_binary, CheckedFromRatioError, Decimal, Deps, DepsMut, Env, MessageInfo, Order,
    QueryResponse, Response, StdResult, Uint64, WasmMsg,
};
use nois_proxy::NoisCallbackMsg;
use rand_core::{RngCore, SeedableRng};
use rand_xoshiro::Xoshiro128PlusPlus;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{LATEST_RESULT, NOIS_PROXY, RESULTS};

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
        ExecuteMsg::EstimatePi { round, job_id } => {
            execute_estimate_pi(deps, env, info, round, job_id)
        }
        ExecuteMsg::Receive(NoisCallbackMsg { id, randomness }) => {
            execute_receive(deps, env, info, id, randomness)
        }
    }
}

pub fn execute_estimate_pi(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    round: Uint64,
    job_id: String,
) -> Result<Response, ContractError> {
    let nois_proxy = NOIS_PROXY.load(deps.storage)?;

    let res = Response::new().add_message(WasmMsg::Execute {
        contract_addr: nois_proxy,
        msg: to_binary(&nois_proxy::ExecuteMsg::GetRound {
            round,
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
    id: String,
    randomness: String,
) -> Result<Response, ContractError> {
    LATEST_RESULT.save(deps.storage, &randomness)?;

    let randomness =
        hex::decode(&randomness).map_err(|_from_hex_err| ContractError::InvalidRandomness)?;
    let randomness: [u8; 32] = randomness
        .try_into()
        .map_err(|_| ContractError::InvalidRandomness)?;

    // Cut first 16 bytes from 32 byte value
    let seed: [u8; 16] = randomness[0..16].try_into().unwrap();

    // A PRNG that is not cryptographically secure.
    // See https://docs.rs/rand/0.8.5/rand/rngs/struct.SmallRng.html
    // where this is used for 32 bit systems.
    // We don't use the SmallRng in order to get the same implementation
    // in unit tests (64 bit dev machines) and the real contract (32 bit Wasm)
    let mut rng = Xoshiro128PlusPlus::from_seed(seed);

    const ROUNDS: u32 = 25_000;
    let mut inside = 0u32;

    let mut buf = [0u8; 16];
    for _round in 0..ROUNDS {
        // get x and y value in [0, 10**18]
        rng.fill_bytes(&mut buf as &mut [u8]);
        let x = u128::from_be_bytes(buf) % 1000000000000000001;
        rng.fill_bytes(&mut buf as &mut [u8]);
        let y = u128::from_be_bytes(buf) % 1000000000000000001;
        let x = Decimal::from_atomics(x, 18).unwrap();
        let y = Decimal::from_atomics(y, 18).unwrap();
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

    RESULTS.save(deps.storage, &id, &estimated_pi.to_string())?;
    LATEST_RESULT.save(deps.storage, &estimated_pi.to_string())?;

    Ok(Response::default())
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<QueryResponse> {
    match msg {
        QueryMsg::Results {} => to_binary(&query_results(deps)?),
        QueryMsg::LatestResult {} => to_binary(&query_latest_result(deps)?),
    }
}

fn query_results(deps: Deps) -> StdResult<Vec<String>> {
    let out: Vec<String> = RESULTS
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| item.map(|(_id, value)| value))
        .collect::<StdResult<_>>()?;
    Ok(out)
}

fn query_latest_result(deps: Deps) -> StdResult<String> {
    let results = LATEST_RESULT.load(deps.storage)?;
    Ok(results)
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
