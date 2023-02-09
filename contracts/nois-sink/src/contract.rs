use cosmwasm_std::{
    ensure_eq, entry_point, to_binary, BankMsg, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo,
    Order, QueryResponse, Response, StdResult,
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
    //Check that the denom is correct
    ensure_eq!(info.funds.len(), 1, ContractError::TooManyOrNoCoins);
    ensure_eq!(info.funds[0].denom, BURN_DENOM, ContractError::WrongDenom);
    let ashes_count = ASHES_COUNT.load(deps.storage).unwrap_or_default();
    ASHES_COUNT.save(deps.storage, &(ashes_count + 1))?;

    let amount = info.funds[0];
    let address = info.sender;
    let timestamp = env.block.time;

    let msg = CosmosMsg::Bank(BankMsg::Burn { amount: info.funds });

    //store the burner Ash
    ASHES.save(
        deps.storage,
        ashes_count + 1,
        &Ash {
            address: address.clone(),
            amount,
            timestamp,
        },
    )?;

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("burnt amount", amount.to_string())
        .add_attribute("burn initiator", address)
        .add_attribute("timestamp", timestamp.to_string()))
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
    use cosmwasm_std::{from_binary, Addr, Attribute, Coin, Timestamp, Uint128};

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
        instantiate(deps.as_mut(), env.to_owned(), info, msg).unwrap();

        let msg = ExecuteMsg::Burn {};
        let info = mock_info("creator", &[Coin::new(1_000, "bitcoin".to_string())]);
        let err = execute(deps.as_mut(), mock_env(), info, msg.to_owned()).unwrap_err();
        assert_eq!(err, ContractError::WrongDenom);
        let info = mock_info(
            "creator",
            &[
                Coin::new(1_000, "unois".to_string()),
                Coin::new(1_000, "bitcoin".to_string()),
            ],
        );
        let err = execute(deps.as_mut(), mock_env(), info, msg.to_owned()).unwrap_err();
        assert_eq!(err, ContractError::TooManyOrNoCoins);
        let info = mock_info("creator", &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg.to_owned()).unwrap_err();
        assert_eq!(err, ContractError::TooManyOrNoCoins);
        let info = mock_info("burner-1", &[Coin::new(1_000, "unois".to_string())]);
        let resp = execute(deps.as_mut(), env.to_owned(), info, msg.to_owned()).unwrap();
        assert_eq!(
            first_attr(&resp.attributes, "burnt amount").unwrap(),
            "1000"
        );
        assert_eq!(
            first_attr(&resp.attributes, "burn initiator").unwrap(),
            "burner-1"
        );
        assert_eq!(
            first_attr(&resp.attributes, "timestamp").unwrap(),
            "1571797419.879305533"
        );

        let info = mock_info("burner-2", &[Coin::new(2_000, "unois".to_string())]);
        execute(deps.as_mut(), env.to_owned(), info, msg.to_owned()).unwrap();
        let info = mock_info("burner-3", &[Coin::new(3_000, "unois".to_string())]);
        execute(deps.as_mut(), env.to_owned(), info, msg.to_owned()).unwrap();
        let info = mock_info("burner-4", &[Coin::new(4_000, "unois".to_string())]);
        execute(deps.as_mut(), env.to_owned(), info, msg.to_owned()).unwrap();
        let info = mock_info("burner-5", &[Coin::new(5_000, "unois".to_string())]);
        execute(deps.as_mut(), env.to_owned(), info, msg.to_owned()).unwrap();
        let info = mock_info("burner-6", &[Coin::new(6_000, "unois".to_string())]);
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
        let ashes_response = ashes.iter().map(|ash| ash.to_owned()).collect::<Vec<Ash>>();
        assert_eq!(
            ashes_response,
            [
                Ash {
                    address: Addr::unchecked("burner-1"),
                    amount: Uint128::new(1000),
                    timestamp: Timestamp::from_nanos(1_571_797_419_879_305_533)
                },
                Ash {
                    address: Addr::unchecked("burner-2"),
                    amount: Uint128::new(2000),
                    timestamp: Timestamp::from_nanos(1_571_797_419_879_305_533)
                },
                Ash {
                    address: Addr::unchecked("burner-3"),
                    amount: Uint128::new(3000),
                    timestamp: Timestamp::from_nanos(1_571_797_419_879_305_533)
                },
                Ash {
                    address: Addr::unchecked("burner-4"),
                    amount: Uint128::new(4000),
                    timestamp: Timestamp::from_nanos(1_571_797_419_879_305_533)
                },
                Ash {
                    address: Addr::unchecked("burner-5"),
                    amount: Uint128::new(5000),
                    timestamp: Timestamp::from_nanos(1_571_797_419_879_305_533)
                },
                Ash {
                    address: Addr::unchecked("burner-6"),
                    amount: Uint128::new(6000),
                    timestamp: Timestamp::from_nanos(1_571_797_419_879_305_533)
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
        let ashes_response = ashes.iter().map(|ash| ash.to_owned()).collect::<Vec<Ash>>();
        assert_eq!(
            ashes_response,
            [
                Ash {
                    address: Addr::unchecked("burner-6"),
                    amount: Uint128::new(6000),
                    timestamp: Timestamp::from_nanos(1_571_797_419_879_305_533)
                },
                Ash {
                    address: Addr::unchecked("burner-5"),
                    amount: Uint128::new(5000),
                    timestamp: Timestamp::from_nanos(1_571_797_419_879_305_533)
                },
                Ash {
                    address: Addr::unchecked("burner-4"),
                    amount: Uint128::new(4000),
                    timestamp: Timestamp::from_nanos(1_571_797_419_879_305_533)
                },
                Ash {
                    address: Addr::unchecked("burner-3"),
                    amount: Uint128::new(3000),
                    timestamp: Timestamp::from_nanos(1_571_797_419_879_305_533)
                },
                Ash {
                    address: Addr::unchecked("burner-2"),
                    amount: Uint128::new(2000),
                    timestamp: Timestamp::from_nanos(1_571_797_419_879_305_533)
                },
                Ash {
                    address: Addr::unchecked("burner-1"),
                    amount: Uint128::new(1000),
                    timestamp: Timestamp::from_nanos(1_571_797_419_879_305_533)
                },
            ]
        );
    }
}
