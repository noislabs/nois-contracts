// Testing nois-drand and nois-oracle interaction

use cosmwasm_std::{
    from_binary, testing::mock_env, to_binary, Addr, Attribute, BalanceResponse, BankQuery, Coin,
    Decimal, HexBinary, Querier, QueryRequest, Uint128, Validator,
};
use cw_multi_test::{App, AppBuilder, ContractWrapper, Executor, StakingInfo};

fn query_balance_native(app: &App, address: &Addr, denom: &str) -> Coin {
    let req: QueryRequest<BankQuery> = QueryRequest::Bank(BankQuery::Balance {
        address: address.to_string(),
        denom: denom.to_string(),
    });
    let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
    let balance: BalanceResponse = from_binary(&res).unwrap();

    balance.amount
}

/// Gets the value of the first attribute with the given key
fn first_attr(data: impl AsRef<[Attribute]>, search_key: &str) -> Option<String> {
    data.as_ref().iter().find_map(|a| {
        if a.key == search_key {
            Some(a.value.clone())
        } else {
            None
        }
    })
}

fn mint_native(
    app: &mut App,
    beneficiary: impl Into<String>,
    denom: impl Into<String>,
    amount: u128,
) {
    app.sudo(cw_multi_test::SudoMsg::Bank(
        cw_multi_test::BankSudo::Mint {
            to_address: beneficiary.into(),
            amount: vec![Coin::new(amount, denom)],
        },
    ))
    .unwrap();
}

#[test]
fn integration_test() {
    // Insantiate a chain mock environment
    let mut app = AppBuilder::new().build(|router, api, storage| {
        router
            .staking
            .setup(
                storage,
                StakingInfo {
                    bonded_denom: "unois".to_string(),
                    unbonding_time: 12,
                    apr: Decimal::percent(12),
                },
            )
            .unwrap();
        let valoper1 = Validator {
            address: "noislabs".to_string(),
            commission: Decimal::percent(1),
            max_commission: Decimal::percent(100),
            max_change_rate: Decimal::percent(1),
        };
        let block = mock_env().block;
        router
            .staking
            .add_validator(api, storage, &block, valoper1)
            .unwrap();
    });

    // Storing nois-drand code
    let code_nois_drand = ContractWrapper::new(
        nois_drand::contract::execute,
        nois_drand::contract::instantiate,
        nois_drand::contract::query,
    );
    let code_id_nois_drand = app.store_code(Box::new(code_nois_drand));

    // Instantiating nois-oracle contract
    let addr_nois_drand = app
        .instantiate_contract(
            code_id_nois_drand,
            Addr::unchecked("owner"),
            &nois_drand::msg::InstantiateMsg {
                manager: "bossman".to_string(),
                incentive_amount: Uint128::new(100_000),
                incentive_denom: "unois".to_string(),
                min_round: 0,
            },
            &[],
            "Nois-Drand",
            None,
        )
        .unwrap();
    let resp: nois_drand::msg::ConfigResponse = app
        .wrap()
        .query_wasm_smart(&addr_nois_drand, &nois_drand::msg::QueryMsg::Config {})
        .unwrap();
    assert_eq!(
        resp,
        nois_drand::msg::ConfigResponse {
            manager: Addr::unchecked("bossman"),
            oracle: None,
            min_round: 0,
            incentive_amount: Uint128::new(100_000),
            incentive_denom: "unois".to_string(),
        }
    );

    //Mint some coins for owner
    mint_native(&mut app, "owner", "unois", 100_000_000);

    // Storing nois-oracle code
    let code_nois_oracle = ContractWrapper::new(
        nois_oracle::contract::execute,
        nois_oracle::contract::instantiate,
        nois_oracle::contract::query,
    );
    let code_id_nois_oracle = app.store_code(Box::new(code_nois_oracle));

    // Instantiating nois-oracle contract
    let addr_nois_oracle = app
        .instantiate_contract(
            code_id_nois_oracle,
            Addr::unchecked("owner"),
            &nois_oracle::msg::InstantiateMsg {},
            &[],
            "Nois-Oracle",
            None,
        )
        .unwrap();
    let resp: nois_oracle::msg::ConfigResponse = app
        .wrap()
        .query_wasm_smart(&addr_nois_oracle, &nois_oracle::msg::QueryMsg::Config {})
        .unwrap();
    //Checking that the contract has been well instantiated with the expected config

    assert_eq!(resp, nois_oracle::msg::ConfigResponse { drand: None });

    // Set oracle address to drand
    app.execute_contract(
        Addr::unchecked("guest"),
        addr_nois_drand.to_owned(),
        &nois_drand::msg::ExecuteMsg::SetOracleAddr {
            addr: addr_nois_oracle.to_string(),
        },
        &[],
    )
    .unwrap();
    let resp: nois_drand::msg::ConfigResponse = app
        .wrap()
        .query_wasm_smart(&addr_nois_drand, &nois_drand::msg::QueryMsg::Config {})
        .unwrap();
    assert_eq!(
        resp,
        nois_drand::msg::ConfigResponse {
            manager: Addr::unchecked("bossman"),
            oracle: Some(addr_nois_oracle.clone()),
            min_round: 0,
            incentive_amount: Uint128::new(100_000),
            incentive_denom: "unois".to_string(),
        }
    );

    // Set drand address to oracle
    app.execute_contract(
        Addr::unchecked("guest"),
        addr_nois_oracle.to_owned(),
        &nois_oracle::msg::ExecuteMsg::SetDrandAddr {
            addr: addr_nois_drand.to_string(),
        },
        &[],
    )
    .unwrap();
    let resp: nois_oracle::msg::ConfigResponse = app
        .wrap()
        .query_wasm_smart(&addr_nois_oracle, &nois_oracle::msg::QueryMsg::Config {})
        .unwrap();
    assert_eq!(
        resp,
        nois_oracle::msg::ConfigResponse {
            drand: Some(addr_nois_drand.clone()),
        }
    );

    // Storing nois-proxy code
    let code_nois_proxy = ContractWrapper::new(
        nois_proxy::contract::execute,
        nois_proxy::contract::instantiate,
        nois_proxy::contract::query,
    );
    let code_id_nois_proxy = app.store_code(Box::new(code_nois_proxy));

    // Instantiating nois-proxy contract
    let addr_nois_proxy = app
        .instantiate_contract(
            code_id_nois_proxy,
            Addr::unchecked("owner"),
            &nois_proxy::msg::InstantiateMsg {
                prices: vec![Coin::new(1_000_000, "unoisx")],
                withdrawal_address: "dao_dao_dao_dao_dao".to_string(),
                test_mode: false,
            },
            &[],
            "Nois-Proxy",
            Some("dao_dao_dao_dao_dao".to_string()),
        )
        .unwrap();
    let resp: nois_proxy::msg::ConfigResponse = app
        .wrap()
        .query_wasm_smart(&addr_nois_proxy, &nois_proxy::msg::QueryMsg::Config {})
        .unwrap();
    //Checking that the contract has been well instantiated with the expected config

    assert_eq!(
        resp,
        nois_proxy::msg::ConfigResponse {
            config: nois_proxy::state::Config {
                prices: vec![Coin::new(1_000_000, "unoisx")],
                withdrawal_address: Addr::unchecked("dao_dao_dao_dao_dao"),
                test_mode: false,
            },
        }
    );

    // register bot
    let msg = nois_drand::msg::ExecuteMsg::RegisterBot {
        moniker: "drand_bot".to_string(),
    };
    app.execute_contract(
        Addr::unchecked("drand_bot"),
        addr_nois_drand.to_owned(),
        &msg,
        &[],
    )
    .unwrap();

    // whitelist bot doesn't work by non admin
    let msg = nois_drand::msg::ExecuteMsg::UpdateAllowlistBots {
        add: vec!["drand_bot".to_string()],
        remove: vec![],
    };
    let err = app
        .execute_contract(
            Addr::unchecked("drand_bot"),
            addr_nois_drand.to_owned(),
            &msg,
            &[],
        )
        .unwrap_err();

    assert!(matches!(
        err.downcast().unwrap(),
        nois_drand::error::ContractError::Unauthorized
    ));

    // add  bot to allow list
    let msg = nois_drand::msg::ExecuteMsg::UpdateAllowlistBots {
        add: vec!["drand_bot".to_string()],
        remove: vec![],
    };
    app.execute_contract(
        Addr::unchecked("bossman"),
        addr_nois_drand.to_owned(),
        &msg,
        &[],
    )
    .unwrap();

    // Add round
    let msg = nois_drand::msg::ExecuteMsg::AddRound {
            // curl -sS https://drand.cloudflare.com/public/72785
            round: 72785,
            previous_signature: HexBinary::from_hex("a609e19a03c2fcc559e8dae14900aaefe517cb55c840f6e69bc8e4f66c8d18e8a609685d9917efbfb0c37f058c2de88f13d297c7e19e0ab24813079efe57a182554ff054c7638153f9b26a60e7111f71a0ff63d9571704905d3ca6df0b031747").unwrap(),
            signature: HexBinary::from_hex("82f5d3d2de4db19d40a6980e8aa37842a0e55d1df06bd68bddc8d60002e8e959eb9cfa368b3c1b77d18f02a54fe047b80f0989315f83b12a74fd8679c4f12aae86eaf6ab5690b34f1fddd50ee3cc6f6cdf59e95526d5a5d82aaa84fa6f181e42").unwrap(),
        };
    let resp = app
        .execute_contract(Addr::unchecked("drand_bot"), addr_nois_drand, &msg, &[])
        .unwrap();

    let wasm = resp.events.iter().find(|ev| ev.ty == "wasm").unwrap();
    // Make sure that there is an incentive for the registered bot
    assert_eq!(
        first_attr(&wasm.attributes, "bot_incentive").unwrap(),
        "100000unois"
    );

    // Check balance nois-oracle
    let balance = query_balance_native(&app, &addr_nois_oracle, "unois").amount;
    assert_eq!(balance, Uint128::new(0));

    // Check balance nois-drand-bot-operator
    // let balance = query_balance_native(&app, &Addr::unchecked("drand_bot"), "unois").amount;
    // assert_eq!(
    //     balance,
    //     Uint128::new(100_000) //incentive
    // );

    //TODO simulte advance many blocks to accumulate some staking rewards
}
