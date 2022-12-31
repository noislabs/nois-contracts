// Testing nois-drand and nois-gateway interaction

use cosmwasm_std::{testing::mock_env, Addr, Coin, Decimal, HexBinary, Uint128, Validator};
use cw_multi_test::{App, AppBuilder, ContractWrapper, Executor, StakingInfo};
use nois_multitest::{first_attr, mint_native, query_balance_native};

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

    // Instantiating nois-drand contract
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
            gateway: None,
            min_round: 0,
            incentive_amount: Uint128::new(100_000),
            incentive_denom: "unois".to_string(),
        }
    );

    //Mint some coins for owner
    mint_native(&mut app, "owner", "unois", 100_000_000);

    // Storing nois-gateway code
    let code_nois_gateway = ContractWrapper::new(
        nois_gateway::contract::execute,
        nois_gateway::contract::instantiate,
        nois_gateway::contract::query,
    );
    let code_id_nois_gateway = app.store_code(Box::new(code_nois_gateway));

    // Instantiating nois-gateway contract
    let addr_nois_gateway = app
        .instantiate_contract(
            code_id_nois_gateway,
            Addr::unchecked("owner"),
            &nois_gateway::msg::InstantiateMsg {},
            &[],
            "Nois-Gateway",
            None,
        )
        .unwrap();
    let resp: nois_gateway::msg::ConfigResponse = app
        .wrap()
        .query_wasm_smart(&addr_nois_gateway, &nois_gateway::msg::QueryMsg::Config {})
        .unwrap();
    //Checking that the contract has been well instantiated with the expected config

    assert_eq!(resp, nois_gateway::msg::ConfigResponse { drand: None });

    // Set gateway address to drand
    app.execute_contract(
        Addr::unchecked("guest"),
        addr_nois_drand.to_owned(),
        &nois_drand::msg::ExecuteMsg::SetGatewayAddr {
            addr: addr_nois_gateway.to_string(),
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
            gateway: Some(addr_nois_gateway.clone()),
            min_round: 0,
            incentive_amount: Uint128::new(100_000),
            incentive_denom: "unois".to_string(),
        }
    );

    // Set drand address to gateway
    app.execute_contract(
        Addr::unchecked("guest"),
        addr_nois_gateway.to_owned(),
        &nois_gateway::msg::ExecuteMsg::SetDrandAddr {
            addr: addr_nois_drand.to_string(),
        },
        &[],
    )
    .unwrap();
    let resp: nois_gateway::msg::ConfigResponse = app
        .wrap()
        .query_wasm_smart(&addr_nois_gateway, &nois_gateway::msg::QueryMsg::Config {})
        .unwrap();
    assert_eq!(
        resp,
        nois_gateway::msg::ConfigResponse {
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
        .query_wasm_smart(addr_nois_proxy, &nois_proxy::msg::QueryMsg::Config {})
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

    // Check balance nois-gateway
    let balance = query_balance_native(&app, &addr_nois_gateway, "unois").amount;
    assert_eq!(balance, Uint128::new(0));

    // Check balance nois-drand-bot-operator
    // let balance = query_balance_native(&app, &Addr::unchecked("drand_bot"), "unois").amount;
    // assert_eq!(
    //     balance,
    //     Uint128::new(100_000) //incentive
    // );

    //TODO simulte advance many blocks to accumulate some staking rewards
}
