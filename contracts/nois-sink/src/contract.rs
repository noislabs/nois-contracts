use cosmwasm_std::{
    ensure_eq, entry_point, to_json_binary, BankMsg, CosmosMsg, Deps, DepsMut, Empty, Env,
    MessageInfo, Order, QueryResponse, Response, StdResult,
};
use cw2::set_contract_version;
use cw_storage_plus::Bound;

use crate::error::ContractError;
use crate::msg::{AshesResponse, ExecuteMsg, InstantiateMsg, QueriedAsh, QueryMsg};
use crate::state::{Ash, ASHES, ASHES_LAST_ID};

/// Constant defining the denom of the Coin to be burnt
const BURN_DENOM: &str = "unois";

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(
        deps.storage,
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
    )?;
    Ok(Response::default())
}

#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, _msg: Empty) -> StdResult<Response> {
    set_contract_version(
        deps.storage,
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
    )?;
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
        ExecuteMsg::BurnBalance {} => execute_burn_balance(deps, info, env),
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<QueryResponse> {
    let response = match msg {
        QueryMsg::AshesAsc { start_after, limit } => {
            to_json_binary(&query_ashes(deps, start_after, limit, Order::Ascending)?)?
        }
        QueryMsg::AshesDesc { start_after, limit } => {
            to_json_binary(&query_ashes(deps, start_after, limit, Order::Descending)?)?
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

    let new_id = ASHES_LAST_ID.may_load(deps.storage)?.unwrap_or_default() + 1;
    ASHES_LAST_ID.save(deps.storage, &new_id)?;

    let time = env.block.time;

    //store the burner Ash
    ASHES.save(
        deps.storage,
        new_id,
        &Ash {
            burner: Some(burner.clone()),
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

fn execute_burn_balance(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
) -> Result<Response, ContractError> {
    if !info.funds.is_empty() {
        return Err(ContractError::NonPayableMessage);
    }
    let contract_balance = deps
        .querier
        .query_balance(&env.contract.address, BURN_DENOM)?;

    if contract_balance.amount.is_zero() {
        return Err(ContractError::NoFundsToBurn);
    }

    let new_id = ASHES_LAST_ID.may_load(deps.storage)?.unwrap_or_default() + 1;
    ASHES_LAST_ID.save(deps.storage, &new_id)?;

    let time = env.block.time;

    //store the burner Ash
    ASHES.save(
        deps.storage,
        new_id,
        &Ash {
            burner: None,
            amount: contract_balance.clone(),
            time,
        },
    )?;

    let msg = CosmosMsg::Bank(BankMsg::Burn {
        amount: vec![contract_balance.clone()],
    });

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("burnt_amount", contract_balance.to_string())
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

    let ashes = ASHES
        .range(deps.storage, low_bound, top_bound, order)
        .take(limit)
        .map(|result| -> StdResult<QueriedAsh> {
            let (key, ash) = result?;
            Ok(QueriedAsh {
                id: key,
                burner: ash.burner,
                amount: ash.amount,
                time: ash.time,
            })
        })
        .collect::<StdResult<Vec<QueriedAsh>>>()?;
    Ok(AshesResponse { ashes })
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::msg::{ExecuteMsg, QueriedAsh};

    use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};
    use cosmwasm_std::{coin, from_json, Attribute, Coin, Timestamp, Uint128};

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

        let creator = deps.api.addr_make("creator");
        let burner1 = deps.api.addr_make("burner-1");
        let burner2 = deps.api.addr_make("burner-2");
        let burner3 = deps.api.addr_make("burner-3");
        let burner4 = deps.api.addr_make("burner-4");

        let info = message_info(&creator, &[]);
        let msg = InstantiateMsg {};
        let env = mock_env();
        instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        let msg = ExecuteMsg::Burn {};
        let info = message_info(&creator, &[coin(1_000, "bitcoin".to_string())]);
        let err = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
        assert_eq!(err, ContractError::WrongDenom);
        let info = message_info(
            &creator,
            &[
                coin(1_000, "unois".to_string()),
                coin(1_000, "bitcoin".to_string()),
            ],
        );
        let err = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
        assert_eq!(err, ContractError::TooManyCoins);
        let info = message_info(&creator, &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
        assert_eq!(err, ContractError::NoCoins);
        let info = message_info(&burner1, &[coin(1_000, "unois".to_string())]);
        let resp = execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap();
        assert_eq!(
            first_attr(&resp.attributes, "burnt_amount").unwrap(),
            "1000unois"
        );
        assert_eq!(
            first_attr(&resp.attributes, "burner").unwrap(),
            burner1.as_str()
        );
        assert_eq!(
            first_attr(&resp.attributes, "time").unwrap(),
            "1571797419.879305533"
        );

        let info = message_info(&burner2, &[coin(2_000, "unois".to_string())]);
        execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap();
        let info = message_info(&burner3, &[coin(3_000, "unois".to_string())]);
        execute(deps.as_mut(), env.clone(), info, msg.clone()).unwrap();
        let info = message_info(&burner4, &[coin(4_000, "unois".to_string())]);
        execute(deps.as_mut(), env, info, msg).unwrap();

        // Test Query Asc
        let AshesResponse { ashes } = from_json(
            query(
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
                QueriedAsh {
                    id: 1,
                    burner: Some(burner1.clone()),
                    amount: coin(1_000, "unois"),
                    time: DEFAULT_TIME
                },
                QueriedAsh {
                    id: 2,
                    burner: Some(burner2.clone()),
                    amount: coin(2_000, "unois"),
                    time: DEFAULT_TIME
                },
                QueriedAsh {
                    id: 3,
                    burner: Some(burner3.clone()),
                    amount: coin(3_000, "unois"),
                    time: DEFAULT_TIME
                },
                QueriedAsh {
                    id: 4,
                    burner: Some(burner4.clone()),
                    amount: coin(4_000, "unois"),
                    time: DEFAULT_TIME
                },
            ]
        );

        // Test Query Desc
        let AshesResponse { ashes } = from_json(
            query(
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
                QueriedAsh {
                    id: 4,
                    burner: Some(burner4.clone()),
                    amount: coin(4_000, "unois"),
                    time: DEFAULT_TIME
                },
                QueriedAsh {
                    id: 3,
                    burner: Some(burner3.clone()),
                    amount: coin(3_000, "unois"),
                    time: DEFAULT_TIME
                },
                QueriedAsh {
                    id: 2,
                    burner: Some(burner2.clone()),
                    amount: coin(2_000, "unois"),
                    time: DEFAULT_TIME
                },
                QueriedAsh {
                    id: 1,
                    burner: Some(burner1.clone()),
                    amount: coin(1_000, "unois"),
                    time: DEFAULT_TIME
                },
            ]
        );
    }

    #[test]
    fn burn_native_works() {
        let mut deps = mock_dependencies();

        let creator = deps.api.addr_make("creator");
        let burner1 = deps.api.addr_make("burner-1");
        let burner4 = deps.api.addr_make("burner-4");
        let joe = deps.api.addr_make("joe");

        let info = message_info(&creator, &[]);
        let msg = InstantiateMsg {};
        let env = mock_env();
        instantiate(deps.as_mut(), env.to_owned(), info, msg).unwrap();

        let msg = ExecuteMsg::BurnBalance {};
        let info = message_info(&creator, &[coin(1_000, "unois".to_string())]);
        let err = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
        assert_eq!(err, ContractError::NonPayableMessage);

        let info = message_info(&creator, &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg.to_owned()).unwrap_err();
        assert_eq!(err, ContractError::NoFundsToBurn);
        let contract = env.contract.address;

        deps.querier.bank.update_balance(
            contract.to_owned(),
            vec![Coin {
                denom: "unois".to_string(),
                amount: Uint128::new(100_000_000),
            }],
        );

        let info = message_info(&burner1, &[]);
        let response = execute(deps.as_mut(), mock_env(), info, msg.to_owned()).unwrap();
        assert_eq!(
            response.messages[0].msg,
            CosmosMsg::Bank(BankMsg::Burn {
                amount: vec![Coin {
                    denom: "unois".to_string(),
                    amount: Uint128::new(100_000_000)
                }]
            })
        );
        // Send 3 burn messages
        for a in [1u128, 2] {
            let msg = ExecuteMsg::Burn {};
            let info = message_info(&joe, &[Coin::new(a, "unois")]);
            execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        }
        let info = message_info(&burner4, &[]);
        deps.querier.bank.update_balance(
            contract,
            vec![Coin {
                denom: "unois".to_string(),
                amount: Uint128::new(5_000),
            }],
        );
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let AshesResponse { ashes } = from_json(
            query(
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
                QueriedAsh {
                    id: 1,
                    burner: None,
                    amount: coin(100_000_000, "unois"),
                    time: DEFAULT_TIME
                },
                QueriedAsh {
                    id: 2,
                    burner: Some(joe.clone()),
                    amount: coin(1, "unois"),
                    time: DEFAULT_TIME
                },
                QueriedAsh {
                    id: 3,
                    burner: Some(joe.clone()),
                    amount: coin(2, "unois"),
                    time: DEFAULT_TIME
                },
                QueriedAsh {
                    id: 4,
                    burner: None,
                    amount: coin(5000, "unois"),
                    time: DEFAULT_TIME
                },
            ]
        );
    }

    #[test]
    fn query_works_for_more_than_10_elements() {
        let mut deps = mock_dependencies();

        let creator = deps.api.addr_make("creator");
        let joe = deps.api.addr_make("joe");

        let info = message_info(&creator, &[]);
        let msg = InstantiateMsg {};
        let env = mock_env();
        instantiate(deps.as_mut(), env, info, msg).unwrap();

        // Send 12 burn messages
        for a in [1u128, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12] {
            let msg = ExecuteMsg::Burn {};
            let info = message_info(&joe, &[Coin::new(a, "unois")]);
            execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        }

        // asc, limit 3
        let AshesResponse { ashes } = from_json(
            query(
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
                QueriedAsh {
                    id: 1,
                    burner: Some(joe.clone()),
                    amount: coin(1, "unois"),
                    time: DEFAULT_TIME
                },
                QueriedAsh {
                    id: 2,
                    burner: Some(joe.clone()),
                    amount: coin(2, "unois"),
                    time: DEFAULT_TIME
                },
                QueriedAsh {
                    id: 3,
                    burner: Some(joe.clone()),
                    amount: coin(3, "unois"),
                    time: DEFAULT_TIME
                },
            ]
        );

        // asc, limit 3, start after 2
        let AshesResponse { ashes } = from_json(
            query(
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
                QueriedAsh {
                    id: 3,
                    burner: Some(joe.clone()),
                    amount: coin(3, "unois"),
                    time: DEFAULT_TIME
                },
                QueriedAsh {
                    id: 4,
                    burner: Some(joe.clone()),
                    amount: coin(4, "unois"),
                    time: DEFAULT_TIME
                },
                QueriedAsh {
                    id: 5,
                    burner: Some(joe.clone()),
                    amount: coin(5, "unois"),
                    time: DEFAULT_TIME
                },
            ]
        );

        // asc, limit None
        let AshesResponse { ashes } = from_json(
            query(
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
                QueriedAsh {
                    id: 1,
                    burner: Some(joe.clone()),
                    amount: coin(1, "unois"),
                    time: DEFAULT_TIME
                },
                QueriedAsh {
                    id: 2,
                    burner: Some(joe.clone()),
                    amount: coin(2, "unois"),
                    time: DEFAULT_TIME
                },
                QueriedAsh {
                    id: 3,
                    burner: Some(joe.clone()),
                    amount: coin(3, "unois"),
                    time: DEFAULT_TIME
                },
                QueriedAsh {
                    id: 4,
                    burner: Some(joe.clone()),
                    amount: coin(4, "unois"),
                    time: DEFAULT_TIME
                },
                QueriedAsh {
                    id: 5,
                    burner: Some(joe.clone()),
                    amount: coin(5, "unois"),
                    time: DEFAULT_TIME
                },
                QueriedAsh {
                    id: 6,
                    burner: Some(joe.clone()),
                    amount: coin(6, "unois"),
                    time: DEFAULT_TIME
                },
                QueriedAsh {
                    id: 7,
                    burner: Some(joe.clone()),
                    amount: coin(7, "unois"),
                    time: DEFAULT_TIME
                },
                QueriedAsh {
                    id: 8,
                    burner: Some(joe.clone()),
                    amount: coin(8, "unois"),
                    time: DEFAULT_TIME
                },
                QueriedAsh {
                    id: 9,
                    burner: Some(joe.clone()),
                    amount: coin(9, "unois"),
                    time: DEFAULT_TIME
                },
                QueriedAsh {
                    id: 10,
                    burner: Some(joe.clone()),
                    amount: coin(10, "unois"),
                    time: DEFAULT_TIME
                },
                QueriedAsh {
                    id: 11,
                    burner: Some(joe.clone()),
                    amount: coin(11, "unois"),
                    time: DEFAULT_TIME
                },
                QueriedAsh {
                    id: 12,
                    burner: Some(joe.clone()),
                    amount: coin(12, "unois"),
                    time: DEFAULT_TIME
                },
            ]
        );

        // desc, limit 3, start after 6
        let AshesResponse { ashes } = from_json(
            query(
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
                QueriedAsh {
                    id: 5,
                    burner: Some(joe.clone()),
                    amount: coin(5, "unois"),
                    time: DEFAULT_TIME
                },
                QueriedAsh {
                    id: 4,
                    burner: Some(joe.clone()),
                    amount: coin(4, "unois"),
                    time: DEFAULT_TIME
                },
                QueriedAsh {
                    id: 3,
                    burner: Some(joe.clone()),
                    amount: coin(3, "unois"),
                    time: DEFAULT_TIME
                },
            ]
        );

        // desc, limit 3, start after 5
        let AshesResponse { ashes } = from_json(
            query(
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
                QueriedAsh {
                    id: 2,
                    burner: Some(joe.clone()),
                    amount: coin(2, "unois"),
                    time: DEFAULT_TIME
                },
                QueriedAsh {
                    id: 1,
                    burner: Some(joe.clone()),
                    amount: coin(1, "unois"),
                    time: DEFAULT_TIME
                },
            ]
        );
    }
}
