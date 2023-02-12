use cosmwasm_std::{
    ensure_eq, entry_point, to_binary, BankMsg, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order,
    QueryResponse, Response, StdResult,
};
use cw_storage_plus::Bound;

use crate::error::ContractError;
use crate::msg::{AshesResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Ash, ASHES, ASHES_COUNT};

/// Constant defining the denom of the Coin to be burnt
const BURN_DENOM: &str = "unois";

#[entry_point]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
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
        ExecuteMsg::Burn {} => execute_burn(deps, info, env),
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<QueryResponse> {
    let response = match msg {
        QueryMsg::AshesAsc { start_after, limit } => {
            to_binary(&query_ashes(deps, start_after, limit, Order::Ascending)?)?
        }
        QueryMsg::AshesDesc { start_after, limit } => {
            to_binary(&query_ashes(deps, start_after, limit, Order::Descending)?)?
        }
    };
    Ok(response)
}

fn execute_burn(deps: DepsMut, info: MessageInfo, env: Env) -> Result<Response, ContractError> {
    let MessageInfo {
        mut funds,
        sender: burner,
    } = info;

    if funds.len() > 1 {
        return Err(ContractError::TooManyCoins);
    }

    // Get first coin and ensure it has the correct denom
    let amount = funds.pop().ok_or(ContractError::NoCoins)?;
    ensure_eq!(amount.denom, BURN_DENOM, ContractError::WrongDenom);

    let ashes_count = ASHES_COUNT.may_load(deps.storage)?.unwrap_or_default();
    ASHES_COUNT.save(deps.storage, &(ashes_count + 1))?;

    let time = env.block.time;

    //store the burner Ash
    ASHES.save(
        deps.storage,
        ashes_count + 1,
        &Ash {
            burner: burner.clone(),
            amount: amount.clone(),
            time,
        },
    )?;

    let msg = CosmosMsg::Bank(BankMsg::Burn {
        amount: vec![amount.clone()],
    });

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("burnt_amount", amount.to_string())
        .add_attribute("burner", burner)
        .add_attribute("time", time.to_string()))
}

fn query_ashes(
    deps: Deps,
    start_after: Option<u32>,
    limit: Option<u32>,
    order: Order,
) -> StdResult<AshesResponse> {
    let limit: usize = limit.unwrap_or(100) as usize;
    let (low_bound, top_bound) = match order {
        Order::Ascending => (start_after.map(Bound::exclusive), None),
        Order::Descending => (None, start_after.map(Bound::exclusive)),
    };

    let ashes: Vec<Ash> = ASHES
        .range(deps.storage, low_bound, top_bound, order)
        .take(limit)
        .map(|result| result.unwrap().1)
        .collect();
    Ok(AshesResponse { ashes })
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::msg::ExecuteMsg;

    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{from_binary, Addr, Attribute, Coin, Timestamp};

    const DEFAULT_TIME: Timestamp = Timestamp::from_nanos(1_571_797_419_879_305_533);

    fn first_attr(data: impl AsRef<[Attribute]>, search_key: &str) -> Option<String> {
        data.as_ref().iter().find_map(|a| {
            if a.key == search_key {
                Some(a.value.clone())
            } else {
                None
            }
        })
    }

    #[test]
    fn burn_works() {
        let mut deps = mock_dependencies();

        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {};
        let env = mock_env();
        instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        let msg = ExecuteMsg::Burn {};
        let info = mock_info("creator", &[Coin::new(1_000, "bitcoin".to_string())]);
        let err = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
        assert_eq!(err, ContractError::WrongDenom);
        let info = mock_info(
            "creator",
            &[
                Coin::new(1_000, "unois".to_string()),
                Coin::new(1_000, "bitcoin".to_string()),
            ],
        );
        let err = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
        assert_eq!(err, ContractError::TooManyCoins);
        let info = mock_info("creator", &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
        assert_eq!(err, ContractError::NoCoins);
        let info = mock_info("burner-1", &[Coin::new(1_000, "unois".to_string())]);
        let resp = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap();
        assert_eq!(
            first_attr(&resp.attributes, "burnt_amount").unwrap(),
            "1000unois"
        );
        assert_eq!(first_attr(&resp.attributes, "burner").unwrap(), "burner-1");
        assert_eq!(
            first_attr(&resp.attributes, "time").unwrap(),
            "1571797419.879305533"
        );

        let info = mock_info("burner-2", &[Coin::new(2_000, "unois".to_string())]);
        execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap();
        let info = mock_info("burner-3", &[Coin::new(3_000, "unois".to_string())]);
        execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap();
        let info = mock_info("burner-4", &[Coin::new(4_000, "unois".to_string())]);
        execute(deps.as_mut(), env, info, msg).unwrap();

        // Test Query Asc
        let AshesResponse { ashes } = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::AshesAsc {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            ashes,
            [
                Ash {
                    burner: Addr::unchecked("burner-1"),
                    amount: Coin::new(1_000, "unois"),
                    time: DEFAULT_TIME
                },
                Ash {
                    burner: Addr::unchecked("burner-2"),
                    amount: Coin::new(2_000, "unois"),
                    time: DEFAULT_TIME
                },
                Ash {
                    burner: Addr::unchecked("burner-3"),
                    amount: Coin::new(3_000, "unois"),
                    time: DEFAULT_TIME
                },
                Ash {
                    burner: Addr::unchecked("burner-4"),
                    amount: Coin::new(4_000, "unois"),
                    time: DEFAULT_TIME
                },
            ]
        );

        // Test Query Desc
        let AshesResponse { ashes } = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::AshesDesc {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            ashes,
            [
                Ash {
                    burner: Addr::unchecked("burner-4"),
                    amount: Coin::new(4_000, "unois"),
                    time: DEFAULT_TIME
                },
                Ash {
                    burner: Addr::unchecked("burner-3"),
                    amount: Coin::new(3_000, "unois"),
                    time: DEFAULT_TIME
                },
                Ash {
                    burner: Addr::unchecked("burner-2"),
                    amount: Coin::new(2_000, "unois"),
                    time: DEFAULT_TIME
                },
                Ash {
                    burner: Addr::unchecked("burner-1"),
                    amount: Coin::new(1_000, "unois"),
                    time: DEFAULT_TIME
                },
            ]
        );
    }

    #[test]
    fn query_works_for_more_than_10_elements() {
        let mut deps = mock_dependencies();

        let info = mock_info("creator", &[]);
        let msg = InstantiateMsg {};
        let env = mock_env();
        instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        // Send 12 burn messages
        for a in [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12] {
            let msg = ExecuteMsg::Burn {};
            let info = mock_info("joe", &[Coin::new(a, "unois")]);
            execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        }

        // asc, limit 3
        let AshesResponse { ashes } = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::AshesAsc {
                    start_after: None,
                    limit: Some(3),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            ashes,
            [
                Ash {
                    burner: Addr::unchecked("joe"),
                    amount: Coin::new(1, "unois"),
                    time: DEFAULT_TIME
                },
                Ash {
                    burner: Addr::unchecked("joe"),
                    amount: Coin::new(2, "unois"),
                    time: DEFAULT_TIME
                },
                Ash {
                    burner: Addr::unchecked("joe"),
                    amount: Coin::new(3, "unois"),
                    time: DEFAULT_TIME
                },
            ]
        );

        // asc, limit 3, start after 2
        let AshesResponse { ashes } = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::AshesAsc {
                    start_after: Some(2),
                    limit: Some(3),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            ashes,
            [
                Ash {
                    burner: Addr::unchecked("joe"),
                    amount: Coin::new(3, "unois"),
                    time: DEFAULT_TIME
                },
                Ash {
                    burner: Addr::unchecked("joe"),
                    amount: Coin::new(4, "unois"),
                    time: DEFAULT_TIME
                },
                Ash {
                    burner: Addr::unchecked("joe"),
                    amount: Coin::new(5, "unois"),
                    time: DEFAULT_TIME
                },
            ]
        );

        // asc, limit None
        let AshesResponse { ashes } = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::AshesAsc {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            ashes,
            [
                Ash {
                    burner: Addr::unchecked("joe"),
                    amount: Coin::new(1, "unois"),
                    time: DEFAULT_TIME
                },
                Ash {
                    burner: Addr::unchecked("joe"),
                    amount: Coin::new(2, "unois"),
                    time: DEFAULT_TIME
                },
                Ash {
                    burner: Addr::unchecked("joe"),
                    amount: Coin::new(3, "unois"),
                    time: DEFAULT_TIME
                },
                Ash {
                    burner: Addr::unchecked("joe"),
                    amount: Coin::new(4, "unois"),
                    time: DEFAULT_TIME
                },
                Ash {
                    burner: Addr::unchecked("joe"),
                    amount: Coin::new(5, "unois"),
                    time: DEFAULT_TIME
                },
                Ash {
                    burner: Addr::unchecked("joe"),
                    amount: Coin::new(6, "unois"),
                    time: DEFAULT_TIME
                },
                Ash {
                    burner: Addr::unchecked("joe"),
                    amount: Coin::new(7, "unois"),
                    time: DEFAULT_TIME
                },
                Ash {
                    burner: Addr::unchecked("joe"),
                    amount: Coin::new(8, "unois"),
                    time: DEFAULT_TIME
                },
                Ash {
                    burner: Addr::unchecked("joe"),
                    amount: Coin::new(9, "unois"),
                    time: DEFAULT_TIME
                },
                Ash {
                    burner: Addr::unchecked("joe"),
                    amount: Coin::new(10, "unois"),
                    time: DEFAULT_TIME
                },
                Ash {
                    burner: Addr::unchecked("joe"),
                    amount: Coin::new(11, "unois"),
                    time: DEFAULT_TIME
                },
                Ash {
                    burner: Addr::unchecked("joe"),
                    amount: Coin::new(12, "unois"),
                    time: DEFAULT_TIME
                },
            ]
        );

        // desc, limit 3, start after 6
        let AshesResponse { ashes } = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::AshesDesc {
                    start_after: Some(6),
                    limit: Some(3),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            ashes,
            [
                Ash {
                    burner: Addr::unchecked("joe"),
                    amount: Coin::new(5, "unois"),
                    time: DEFAULT_TIME
                },
                Ash {
                    burner: Addr::unchecked("joe"),
                    amount: Coin::new(4, "unois"),
                    time: DEFAULT_TIME
                },
                Ash {
                    burner: Addr::unchecked("joe"),
                    amount: Coin::new(3, "unois"),
                    time: DEFAULT_TIME
                },
            ]
        );

        // desc, limit 3, start after 5
        let AshesResponse { ashes } = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::AshesDesc {
                    start_after: Some(3),
                    limit: Some(5),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            ashes,
            [
                Ash {
                    burner: Addr::unchecked("joe"),
                    amount: Coin::new(2, "unois"),
                    time: DEFAULT_TIME
                },
                Ash {
                    burner: Addr::unchecked("joe"),
                    amount: Coin::new(1, "unois"),
                    time: DEFAULT_TIME
                },
            ]
        );
    }
}
