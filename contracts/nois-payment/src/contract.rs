use cosmwasm_std::{
    ensure_eq, entry_point, to_binary, BankMsg, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    QueryResponse, Response, StdResult, Uint128, WasmMsg,
};

use crate::error::ContractError;
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, NoisSinkExecuteMsg, QueryMsg};
use crate::state::{Config, CONFIG};

/// Constant defining the denom of the Coin to be used for payment
const PAYMENT_DENOM: &str = "unois";

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let nois_sink_addr = deps
        .api
        .addr_validate(&msg.nois_sink)
        .map_err(|_| ContractError::InvalidAddress)?;
    let nois_com_pool_addr = deps
        .api
        .addr_validate(&msg.nois_com_pool_addr)
        .map_err(|_| ContractError::InvalidAddress)?;
    CONFIG.save(
        deps.storage,
        &Config {
            community_pool: nois_com_pool_addr,
            sink: nois_sink_addr,
            gateway: info.sender.clone(),
        },
    )?;
    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("nois_sink", msg.nois_sink)
        .add_attribute("nois_community_pool", msg.nois_com_pool_addr)
        .add_attribute("nois_gateway", info.sender))
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

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<QueryResponse> {
    let response = match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?)?,
    };
    Ok(response)
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config)
}

fn execute_pay(
    deps: DepsMut,
    info: MessageInfo,
    _env: Env,
    burn: Uint128,
    community_pool: Uint128,
    relayer: (String, Uint128),
) -> Result<Response, ContractError> {
    let funds = info.funds;

    // Make sure the caller is gateway to make sure malicious people can't drain someone else's payment balance
    let gateway = CONFIG.load(deps.storage).unwrap().gateway;
    ensure_eq!(info.sender, gateway, ContractError::Unauthorized);

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
        contract_addr: CONFIG.load(deps.storage).unwrap().sink.to_string(),
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
            to_address: CONFIG
                .load(deps.storage)
                .unwrap()
                .community_pool
                .to_string(),
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
mod tests {

    use crate::msg::{ConfigResponse, QueryMsg};

    use super::*;
    use cosmwasm_std::{
        coins, from_binary,
        testing::{mock_dependencies, mock_env, mock_info},
        Addr, Uint128,
    };

    const NOIS_SINK: &str = "sink";
    const NOIS_COMMUNITY_POOL: &str = "community_pool";
    const NOIS_GATEWAY: &str = "nois-gateway";

    #[test]
    fn instantiate_works() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            nois_sink: NOIS_SINK.to_string(),
            nois_com_pool_addr: NOIS_COMMUNITY_POOL.to_string(),
        };
        let info = mock_info(NOIS_GATEWAY, &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        let config: ConfigResponse =
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
        assert_eq!(
            config,
            ConfigResponse {
                community_pool: Addr::unchecked(NOIS_COMMUNITY_POOL),
                sink: Addr::unchecked(NOIS_SINK),
                gateway: Addr::unchecked(NOIS_GATEWAY),
            }
        );
    }

    #[test]
    fn cannot_send_funds() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            nois_sink: NOIS_SINK.to_string(),
            nois_com_pool_addr: NOIS_COMMUNITY_POOL.to_string(),
        };
        let info = mock_info(NOIS_GATEWAY, &[]);
        let _result = instantiate(deps.as_mut(), mock_env(), info, msg);

        let info = mock_info(NOIS_GATEWAY, &coins(12345, "unoisx"));
        let msg = ExecuteMsg::Pay {
            burn: Uint128::new(500_000),
            community_pool: Uint128::new(450_000),
            relayer: ("some-relayer".to_string(), Uint128::new(50_000)),
        };

        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert!(matches!(err, ContractError::DontSendFunds));
    }
    #[test]
    fn only_gateway_can_pay() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            nois_sink: NOIS_SINK.to_string(),
            nois_com_pool_addr: NOIS_COMMUNITY_POOL.to_string(),
        };
        let info = mock_info(NOIS_GATEWAY, &[]);
        let _result = instantiate(deps.as_mut(), mock_env(), info, msg);

        let info = mock_info("a-malicious-person", &[]);
        let msg = ExecuteMsg::Pay {
            burn: Uint128::new(500_000),
            community_pool: Uint128::new(450_000),
            relayer: ("some-relayer".to_string(), Uint128::new(50_000)),
        };

        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized));
    }

    #[test]
    fn fund_send_works() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            nois_sink: NOIS_SINK.to_string(),
            nois_com_pool_addr: NOIS_COMMUNITY_POOL.to_string(),
        };
        let info = mock_info(NOIS_GATEWAY, &[]);
        let _result = instantiate(deps.as_mut(), mock_env(), info, msg);

        let info = mock_info(NOIS_GATEWAY, &[]);
        let msg = ExecuteMsg::Pay {
            burn: Uint128::new(500_000),
            community_pool: Uint128::new(450_000),
            relayer: ("some-relayer".to_string(), Uint128::new(50_000)),
        };

        let response = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(response.messages.len(), 3); // 3 because we send funds to 3 different addresses (relayer + com_pool + sink)

        // clippy linting  doesnt like this

        //  assert_eq!(
        //      response.messages[0].msg,
        //      CosmosMsg::Wasm(WasmMsg::Execute {
        //          contract_addr: "sink".to_string(),
        //          msg: cosmwasm_std::Binary {
        //              0: vec![123, 34, 98, 117, 114, 110, 34, 58, 123, 125, 125], // {"burn":{}}
        //          },
        //          funds: vec![Coin::new(500_000, "unois")],
        //      })
        //  );
        //
        assert_eq!(
            response.messages[1].msg,
            CosmosMsg::Bank(BankMsg::Send {
                to_address: "some-relayer".to_string(),
                amount: coins(50_000, "unois"),
            })
        );
        assert_eq!(
            response.messages[2].msg,
            CosmosMsg::Bank(BankMsg::Send {
                to_address: "community_pool".to_string(),
                amount: coins(450_000, "unois"),
            })
        );
    }
}
