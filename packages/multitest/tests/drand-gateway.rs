// Testing nois-drand and nois-gateway interaction

use cosmwasm_std::{coin, testing::mock_env, Decimal, HexBinary, Timestamp, Uint128, Validator};
use cw_multi_test::{AppBuilder, ContractWrapper, Executor, StakingInfo};
use nois_multitest::{addr, first_attr, mint_native, payment_initial, query_balance_native};

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
        let valoper1 = Validator::new(
            addr("noislabs").to_string(), // TODO: this should not be an account address
            Decimal::percent(1),
            Decimal::percent(100),
            Decimal::percent(1),
        );
        let block = mock_env().block;
        router
            .staking
            .add_validator(api, storage, &block, valoper1)
            .unwrap();
    });

    let owner = addr("owner");
    let bossman = addr("bossman");
    let manager = addr("manager");
    let sink = addr("sink");

    // Mint 1000 NOIS for owner
    mint_native(&mut app, &owner, "unois", 1_000_000_000);

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
            owner.clone(),
            &nois_drand::msg::InstantiateMsg {
                manager: bossman.to_string(),
                incentive_point_price: Uint128::new(1_500),
                incentive_denom: "unois".to_string(),
                min_round: 0,
            },
            &[coin(600_000_000, "unois")],
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
            manager: bossman.clone(),
            gateway: None,
            min_round: 0,
            incentive_point_price: Uint128::new(1_500),
            incentive_denom: "unois".to_string(),
        }
    );

    // Storing nois-payment code
    let code_nois_payment = ContractWrapper::new(
        nois_payment::contract::execute,
        nois_payment::contract::instantiate,
        nois_payment::contract::query,
    );
    let code_id_nois_payment = app.store_code(Box::new(code_nois_payment));

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
            owner.clone(),
            &nois_gateway::msg::InstantiateMsg {
                manager: manager.to_string(),
                price: coin(1, "unois"),
                payment_code_id: code_id_nois_payment,
                payment_initial_funds: payment_initial(),
                sink: sink.to_string(),
            },
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

    assert_eq!(
        resp,
        nois_gateway::msg::ConfigResponse {
            drand: None,
            trusted_sources: None,
            manager: manager.clone(),
            price: coin(1, "unois"),
            payment_code_id: code_id_nois_payment,
            payment_initial_funds: payment_initial(),
            sink: sink.clone(),
        }
    );

    // Set gateway address to drand
    app.execute_contract(
        bossman.clone(),
        addr_nois_drand.to_owned(),
        &nois_drand::msg::ExecuteMsg::SetConfig {
            manager: None,
            gateway: Some(addr_nois_gateway.to_string()),
            min_round: None,
            incentive_point_price: None,
            incentive_denom: None,
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
            manager: bossman.clone(),
            gateway: Some(addr_nois_gateway.clone()),
            min_round: 0,
            incentive_point_price: Uint128::new(1_500),
            incentive_denom: "unois".to_string(),
        }
    );

    // Set drand address to gateway
    app.execute_contract(
        manager.clone(),
        addr_nois_gateway.to_owned(),
        &nois_gateway::msg::ExecuteMsg::SetConfig {
            manager: None,
            price: None,
            drand_addr: Some(addr_nois_drand.to_string()),
            trusted_sources: Some(vec![addr_nois_drand.to_string()]),
            payment_initial_funds: None,
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
            trusted_sources: Some(vec![addr_nois_drand.clone()]),
            manager: manager.clone(),
            price: coin(1, "unois"),
            payment_code_id: code_id_nois_payment,
            payment_initial_funds: payment_initial(),
            sink: sink.clone(),
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
            owner.clone(),
            &nois_proxy::msg::InstantiateMsg {
                manager: Some(manager.to_string()),
                prices: vec![coin(1_000_000, "unoisx")],
                test_mode: None,
                callback_gas_limit: 500_000,
                mode: nois_proxy::state::OperationalMode::Funded {},
                allowlist_enabled: None,
                allowlist: None,
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

    let instantiation_time = mock_env().block.time;
    assert_eq!(
        resp,
        nois_proxy::msg::ConfigResponse {
            config: nois_proxy::state::Config {
                manager: Some(manager.clone()),
                prices: vec![coin(1_000_000, "unoisx")],
                test_mode: false,
                callback_gas_limit: 500_000,
                payment: None,
                nois_beacon_price: Uint128::zero(),
                nois_beacon_price_updated: Timestamp::from_seconds(0),
                mode: nois_proxy::state::OperationalMode::Funded {},
                allowlist_enabled: Some(false),
                min_after: Some(instantiation_time),
                max_after: Some(instantiation_time.plus_seconds(10 * 365 * 24 * 3600)),
            },
        }
    );

    let bot1 = addr("drand_bot_1");
    let bot2 = addr("drand_bot_two");
    let bot3 = addr("drand_bot_three33333");
    let bot4 = addr("drand_bot_4");
    let bot5 = addr("drand_bot_5");
    let bot6 = addr("drand_bot_six_");
    let bot7 = addr("drand_bot_7");
    let bot8 = addr("drand_bot_8");
    let new_bot = addr("new_bot");

    // register bots
    let msg = nois_drand::msg::ExecuteMsg::RegisterBot {
        moniker: "bot1".to_string(),
    };
    app.execute_contract(bot1.clone(), addr_nois_drand.to_owned(), &msg, &[])
        .unwrap();
    // register bot 2
    let msg = nois_drand::msg::ExecuteMsg::RegisterBot {
        moniker: "bot2".to_string(),
    };
    app.execute_contract(bot2.clone(), addr_nois_drand.to_owned(), &msg, &[])
        .unwrap();
    // register bot 3
    let msg = nois_drand::msg::ExecuteMsg::RegisterBot {
        moniker: "bot3".to_string(),
    };
    app.execute_contract(bot3.clone(), addr_nois_drand.to_owned(), &msg, &[])
        .unwrap();
    // register bot 4
    let msg = nois_drand::msg::ExecuteMsg::RegisterBot {
        moniker: "bot4".to_string(),
    };
    app.execute_contract(bot4.clone(), addr_nois_drand.to_owned(), &msg, &[])
        .unwrap();
    // register bot 5
    let msg = nois_drand::msg::ExecuteMsg::RegisterBot {
        moniker: "bot5".to_string(),
    };
    app.execute_contract(bot5.clone(), addr_nois_drand.to_owned(), &msg, &[])
        .unwrap();
    // register bot 6
    let msg = nois_drand::msg::ExecuteMsg::RegisterBot {
        moniker: "bot6".to_string(),
    };
    app.execute_contract(bot6.clone(), addr_nois_drand.to_owned(), &msg, &[])
        .unwrap();
    // register bot 7
    let msg = nois_drand::msg::ExecuteMsg::RegisterBot {
        moniker: "bot7".to_string(),
    };
    app.execute_contract(bot7.clone(), addr_nois_drand.to_owned(), &msg, &[])
        .unwrap();
    // register bot 8
    let msg = nois_drand::msg::ExecuteMsg::RegisterBot {
        moniker: "bot8".to_string(),
    };
    app.execute_contract(bot8.clone(), addr_nois_drand.to_owned(), &msg, &[])
        .unwrap();

    // whitelist bot doesn't work by non admin
    let msg = nois_drand::msg::ExecuteMsg::UpdateAllowlistBots {
        add: vec![new_bot.to_string()],
        remove: vec![],
    };
    let err = app
        .execute_contract(new_bot, addr_nois_drand.to_owned(), &msg, &[])
        .unwrap_err();

    assert!(matches!(
        err.downcast().unwrap(),
        nois_drand::error::ContractError::Unauthorized
    ));

    // add  bot to allow list
    let msg = nois_drand::msg::ExecuteMsg::UpdateAllowlistBots {
        add: vec![
            bot1.to_string(),
            bot2.to_string(),
            bot3.to_string(),
            bot4.to_string(),
            bot5.to_string(),
            bot6.to_string(),
            bot7.to_string(),
            bot8.to_string(),
        ],
        remove: vec![],
    };
    app.execute_contract(bossman.clone(), addr_nois_drand.to_owned(), &msg, &[])
        .unwrap();

    // Add round
    const ROUND: u64 = 72775;
    // curl -sS https://drand.cloudflare.com/dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493/public/72775
    const SIGNATURE: &str = "973ae0dd58e53c7ca80952ee26e0565627dd61cc5ded60b20d2d846e5354d2aec13d08a2bfbc240c794993d16a0dae90";
    let msg = nois_drand::msg::ExecuteMsg::AddRound {
        round: ROUND,
        signature: HexBinary::from_hex(SIGNATURE).unwrap(),
    };
    let resp = app
        .execute_contract(bot1.clone(), addr_nois_drand.clone(), &msg, &[])
        .unwrap();

    let wasm = resp.events.iter().find(|ev| ev.ty == "wasm").unwrap();
    // Make sure that there is an incentive for the registered bot
    assert_eq!(
        first_attr(&wasm.attributes, "reward_points").unwrap(),
        "50" // 35 verification + 15 fast
    );
    assert_eq!(
        first_attr(&wasm.attributes, "reward_payout").unwrap(),
        "75000unois" // (35 verification + 15 fast) * 1_500
    );
    // Add round 2nd submission
    let msg = nois_drand::msg::ExecuteMsg::AddRound {
        round: ROUND,
        signature: HexBinary::from_hex(SIGNATURE).unwrap(),
    };
    let resp = app
        .execute_contract(bot2.clone(), addr_nois_drand.clone(), &msg, &[])
        .unwrap();

    let wasm = resp.events.iter().find(|ev| ev.ty == "wasm").unwrap();
    // Make sure that there is an incentive for the registered bot
    assert_eq!(
        first_attr(&wasm.attributes, "reward_points").unwrap(),
        "50" // 35 verification + 15 fast
    );
    assert_eq!(
        first_attr(&wasm.attributes, "reward_payout").unwrap(),
        "75000unois" // (35 verification + 15 fast) * 1_500
    );
    // Add round 3rd submission
    let msg = nois_drand::msg::ExecuteMsg::AddRound {
        round: ROUND,
        signature: HexBinary::from_hex(SIGNATURE).unwrap(),
    };
    let resp = app
        .execute_contract(bot3.clone(), addr_nois_drand.clone(), &msg, &[])
        .unwrap();

    let wasm = resp.events.iter().find(|ev| ev.ty == "wasm").unwrap();
    // Make sure that there is an incentive for the registered bot
    assert_eq!(
        first_attr(&wasm.attributes, "reward_points").unwrap(),
        "50" // 35 verification + 15 fast
    );
    assert_eq!(
        first_attr(&wasm.attributes, "reward_payout").unwrap(),
        "75000unois" // (35 verification + 15 fast) * 1_500
    );
    // Add round 4th submission
    let msg = nois_drand::msg::ExecuteMsg::AddRound {
        round: ROUND,
        signature: HexBinary::from_hex(SIGNATURE).unwrap(),
    };
    let resp = app
        .execute_contract(bot4.clone(), addr_nois_drand.clone(), &msg, &[])
        .unwrap();

    let wasm = resp.events.iter().find(|ev| ev.ty == "wasm").unwrap();
    // Make sure that there is an incentive for the registered bot
    assert_eq!(
        first_attr(&wasm.attributes, "reward_points").unwrap(),
        "15" // 0 verification + 15 fast
    );
    assert_eq!(
        first_attr(&wasm.attributes, "reward_payout").unwrap(),
        "22500unois" // (0 verification + 15 fast) * 1_500
    );
    // Add round 5th submission
    let msg = nois_drand::msg::ExecuteMsg::AddRound {
        round: ROUND,
        signature: HexBinary::from_hex(SIGNATURE).unwrap(),
    };
    let resp = app
        .execute_contract(bot5.clone(), addr_nois_drand.clone(), &msg, &[])
        .unwrap();

    let wasm = resp.events.iter().find(|ev| ev.ty == "wasm").unwrap();
    // Make sure that there is an incentive for the registered bot
    assert_eq!(
        first_attr(&wasm.attributes, "reward_points").unwrap(),
        "15" // 0 verification + 15 fast
    );
    assert_eq!(
        first_attr(&wasm.attributes, "reward_payout").unwrap(),
        "22500unois" // (0 verification + 15 fast) * 1_500
    );
    // Add round 6th submission
    let msg = nois_drand::msg::ExecuteMsg::AddRound {
        round: ROUND,
        signature: HexBinary::from_hex(SIGNATURE).unwrap(),
    };
    let resp = app
        .execute_contract(bot6.clone(), addr_nois_drand.clone(), &msg, &[])
        .unwrap();

    let wasm = resp.events.iter().find(|ev| ev.ty == "wasm").unwrap();
    // Make sure that there is an incentive for the registered bot
    assert_eq!(
        first_attr(&wasm.attributes, "reward_points").unwrap(),
        "15" // 0 verification + 15 fast
    );
    assert_eq!(
        first_attr(&wasm.attributes, "reward_payout").unwrap(),
        "22500unois" // (0 verification + 15 fast) * 1_500
    );
    // Add round 7th submission
    let msg = nois_drand::msg::ExecuteMsg::AddRound {
        round: ROUND,
        signature: HexBinary::from_hex(SIGNATURE).unwrap(),
    };
    let resp = app
        .execute_contract(bot7.clone(), addr_nois_drand.clone(), &msg, &[])
        .unwrap();

    let wasm = resp.events.iter().find(|ev| ev.ty == "wasm").unwrap();
    // Make sure that there is no incentive for this bot because it didn't do the verification and it was slow
    // i.e. enough drandbots have already verified this round.
    assert_eq!(first_attr(&wasm.attributes, "reward_points").unwrap(), "0");
    assert_eq!(
        first_attr(&wasm.attributes, "reward_payout").unwrap(),
        "0unois"
    );

    // Add round 8th submission: invalid siganture
    //
    // Check that when a submission has been verified in previous txs by enough other bots
    // and when a new bot brings a submission that won't go through verification. It should fail if it
    // is different from the randomness already registered on contract state for that round.
    let msg = nois_drand::msg::ExecuteMsg::AddRound {
        round: ROUND,
        signature: HexBinary::from_hex("886832ac1b059709a8966347fc447773e15ceff1eada944504fa541ab71c1d1c9ff4f2bbc69f90669a0cf936d018ab52").unwrap(),
    };
    let err = app
        .execute_contract(bot8.clone(), addr_nois_drand, &msg, &[])
        .unwrap_err();

    assert!(matches!(
        err.downcast().unwrap(),
        nois_drand::error::ContractError::SignatureDoesNotMatchState
    ));

    // Check balance nois-gateway
    let balance = query_balance_native(&app, &addr_nois_gateway, "unois").amount;
    assert_eq!(balance, Uint128::new(0));
}
