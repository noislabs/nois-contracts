use anything::Anything;
use cosmwasm_std::{
    ensure_eq, entry_point, to_binary, BankMsg, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    QueryResponse, Response, StdResult, WasmMsg,
};

use crate::error::ContractError;
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, NoisSinkExecuteMsg, QueryMsg};
use crate::state::{Config, CONFIG};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let nois_sink_addr = deps
        .api
        .addr_validate(&msg.sink)
        .map_err(|_| ContractError::InvalidAddress)?;
    let nois_com_pool_addr = deps
        .api
        .addr_validate(&msg.community_pool)
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
        .add_attribute("nois_sink", msg.sink)
        .add_attribute("nois_community_pool", msg.community_pool)
        .add_attribute("nois_gateway", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
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

#[cfg_attr(not(feature = "library"), entry_point)]
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
    env: Env,
    burn: Coin,
    community_pool: Coin,
    relayer: (String, Coin),
) -> Result<Response, ContractError> {
    let funds = info.funds;
    let config = CONFIG.load(deps.storage).unwrap();

    // Make sure the caller is gateway to make sure malicious people can't drain someone else's payment balance
    ensure_eq!(info.sender, config.gateway, ContractError::Unauthorized);

    // Check there are no funds. Not a payable Msg
    if !funds.is_empty() {
        return Err(ContractError::DontSendFunds);
    }
    // Check relayer addr is valid
    deps.api
        .addr_validate(relayer.0.as_str())
        .map_err(|_| ContractError::InvalidAddress)?;

    let mut out_msgs: Vec<CosmosMsg> = Vec::with_capacity(3);

    // Burn
    if !burn.amount.is_zero() {
        out_msgs.push(
            WasmMsg::Execute {
                contract_addr: config.sink.to_string(),
                msg: to_binary(&NoisSinkExecuteMsg::Burn {})?,
                funds: vec![burn.clone()],
            }
            .into(),
        );
    }

    // Send to relayer
    if !relayer.1.amount.is_zero() {
        out_msgs.push(
            BankMsg::Send {
                to_address: relayer.0.to_owned(),
                amount: vec![relayer.1.clone()],
            }
            .into(),
        );
    }

    // Send to community pool
    if !community_pool.amount.is_zero() {
        // Coin: https://github.com/cosmos/cosmos-sdk/blob/v0.45.15/proto/cosmos/base/v1beta1/coin.proto#L14-L19
        // MsgFundCommunityPool: https://github.com/cosmos/cosmos-sdk/blob/v0.45.15/proto/cosmos/distribution/v1beta1/tx.proto#L69-L76
        let coin = Anything::new()
            .append_bytes(1, &community_pool.denom)
            .append_bytes(2, community_pool.amount.to_string());
        let msg_fund_community_pool = Anything::new()
            .append_anything(1, &coin)
            .append_bytes(2, env.contract.address.as_bytes())
            .into_vec();
        out_msgs.push(CosmosMsg::Stargate {
            type_url: "cosmos.distribution.v1beta1.MsgFundCommunityPool".to_string(),
            value: msg_fund_community_pool.into(),
        });
    }

    Ok(Response::new()
        .add_messages(out_msgs)
        .add_attribute("burnt", burn.to_string())
        .add_attribute("relayer_reward", relayer.1.to_string())
        .add_attribute("relayer_address", relayer.0)
        .add_attribute("sent_to_community_pool", community_pool.to_string()))
}

#[cfg(test)]
mod tests {

    use crate::msg::{ConfigResponse, QueryMsg};

    use super::*;
    use cosmwasm_std::{
        coins, from_binary,
        testing::{mock_dependencies, mock_env, mock_info},
        Addr, Attribute, Binary, Uint128,
    };

    const NOIS_SINK: &str = "sink";
    const NOIS_COMMUNITY_POOL: &str = "community_pool";
    const NOIS_GATEWAY: &str = "nois-gateway";

    /// Gets the value of the first attribute with the given key
    pub fn first_attr(data: impl AsRef<[Attribute]>, search_key: &str) -> Option<String> {
        data.as_ref().iter().find_map(|a| {
            if a.key == search_key {
                Some(a.value.clone())
            } else {
                None
            }
        })
    }

    #[test]
    fn instantiate_works() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            sink: NOIS_SINK.to_string(),
            community_pool: NOIS_COMMUNITY_POOL.to_string(),
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
            sink: NOIS_SINK.to_string(),
            community_pool: NOIS_COMMUNITY_POOL.to_string(),
        };
        let info = mock_info(NOIS_GATEWAY, &[]);
        let _result = instantiate(deps.as_mut(), mock_env(), info, msg);

        let info = mock_info(NOIS_GATEWAY, &coins(12345, "unoisx"));
        let msg = ExecuteMsg::Pay {
            burn: Coin {
                denom: "unois".to_string(),
                amount: Uint128::new(500_000),
            },
            community_pool: Coin {
                denom: "unois".to_string(),
                amount: Uint128::new(450_000),
            },
            relayer: (
                "some-relayer".to_string(),
                Coin {
                    denom: "unois".to_string(),
                    amount: Uint128::new(50_000),
                },
            ),
        };

        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert!(matches!(err, ContractError::DontSendFunds));
    }

    #[test]
    fn only_gateway_can_pay() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            sink: NOIS_SINK.to_string(),
            community_pool: NOIS_COMMUNITY_POOL.to_string(),
        };
        let info = mock_info(NOIS_GATEWAY, &[]);
        let _result = instantiate(deps.as_mut(), mock_env(), info, msg);

        let info = mock_info("a-malicious-person", &[]);
        let msg = ExecuteMsg::Pay {
            burn: Coin {
                denom: "unois".to_string(),
                amount: Uint128::new(500_000),
            },
            community_pool: Coin {
                denom: "unois".to_string(),
                amount: Uint128::new(450_000),
            },
            relayer: (
                "some-relayer".to_string(),
                Coin {
                    denom: "unois".to_string(),
                    amount: Uint128::new(50_000),
                },
            ),
        };

        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized));
    }

    #[test]
    fn pay_fund_send_works() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            sink: NOIS_SINK.to_string(),
            community_pool: NOIS_COMMUNITY_POOL.to_string(),
        };
        let info = mock_info(NOIS_GATEWAY, &[]);
        let _result = instantiate(deps.as_mut(), mock_env(), info, msg);

        let info = mock_info(NOIS_GATEWAY, &[]);
        let msg = ExecuteMsg::Pay {
            burn: Coin {
                denom: "unois".to_string(),
                amount: Uint128::new(500_000),
            },
            community_pool: Coin {
                denom: "unois".to_string(),
                amount: Uint128::new(450_000),
            },
            relayer: (
                "some-relayer".to_string(),
                Coin {
                    denom: "unois".to_string(),
                    amount: Uint128::new(50_000),
                },
            ),
        };

        let response = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(response.messages.len(), 3); // 3 because we send funds to 3 different addresses (sink + relayer + com_pool)
        assert_eq!(
            response.messages[0].msg,
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "sink".to_string(),
                msg: Binary::from(br#"{"burn":{}}"#),
                funds: vec![Coin::new(500_000, "unois")],
            })
        );
        assert_eq!(
            response.messages[1].msg,
            CosmosMsg::Bank(BankMsg::Send {
                to_address: "some-relayer".to_string(),
                amount: coins(50_000, "unois"),
            })
        );
        assert!(matches!(
            response.messages[2].msg,
            CosmosMsg::Stargate { .. }
        ));
        assert_eq!(
            first_attr(&response.attributes, "burnt").unwrap(),
            "500000unois"
        );
        assert_eq!(
            first_attr(&response.attributes, "relayer_reward").unwrap(),
            "50000unois"
        );
        assert_eq!(
            first_attr(&response.attributes, "sent_to_community_pool").unwrap(),
            "450000unois"
        );

        // Zero amount is supported
        let info = mock_info(NOIS_GATEWAY, &[]);
        let msg = ExecuteMsg::Pay {
            burn: Coin::new(0, "unois"),
            community_pool: Coin::new(0, "unois"),
            relayer: ("some-relayer".to_string(), Coin::new(0, "unois")),
        };
        let response = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        // 0 because sink does not like empty funds array and bank send does not like zero coins
        assert_eq!(response.messages.len(), 0);
        assert_eq!(first_attr(&response.attributes, "burnt").unwrap(), "0unois");
        assert_eq!(
            first_attr(&response.attributes, "relayer_reward").unwrap(),
            "0unois"
        );
        assert_eq!(
            first_attr(&response.attributes, "sent_to_community_pool").unwrap(),
            "0unois"
        );
    }
}
