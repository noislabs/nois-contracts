// Testing nois-drand and nois-gateway interaction

use cosmwasm_std::{
    testing::mock_env, Addr, Coin, Decimal, HexBinary, Timestamp, Uint128, Validator,
};
use cw_multi_test::{AppBuilder, ContractWrapper, Executor, StakingInfo};
use nois_multitest::{first_attr, mint_native, payment_initial, query_balance_native};

const SINK: &str = "sink";

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

    // Mint 1000 NOIS for owner
    mint_native(&mut app, "owner", "unois", 1_000_000_000);

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
                incentive_point_price: Uint128::new(1_500),
                incentive_denom: "unois".to_string(),
                min_round: 0,
            },
            &[Coin::new(600_000_000, "unois")],
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
            Addr::unchecked("owner"),
            &nois_gateway::msg::InstantiateMsg {
                manager: "manager".to_string(),
                price: Coin::new(1, "unois"),
                payment_code_id: code_id_nois_payment,
                payment_initial_funds: payment_initial(),
                sink: SINK.to_string(),
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
            manager: Addr::unchecked("manager"),
            price: Coin::new(1, "unois"),
            payment_code_id: code_id_nois_payment,
            payment_initial_funds: payment_initial(),
            sink: Addr::unchecked(SINK),
        }
    );

    // Set gateway address to drand
    app.execute_contract(
        Addr::unchecked("bossman"),
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
            manager: Addr::unchecked("bossman"),
            gateway: Some(addr_nois_gateway.clone()),
            min_round: 0,
            incentive_point_price: Uint128::new(1_500),
            incentive_denom: "unois".to_string(),
        }
    );

    // Set drand address to gateway
    app.execute_contract(
        Addr::unchecked("manager"),
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
            manager: Addr::unchecked("manager"),
            price: Coin::new(1, "unois"),
            payment_code_id: code_id_nois_payment,
            payment_initial_funds: payment_initial(),
            sink: Addr::unchecked(SINK),
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
                manager: Some("manager".to_string()),
                prices: vec![Coin::new(1_000_000, "unoisx")],
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
                manager: Some(Addr::unchecked("manager")),
                prices: vec![Coin::new(1_000_000, "unoisx")],
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

    const BOT1: &str = "drand_bot_one";
    const BOT2: &str = "drand_bot_2";
    const BOT3: &str = "drand_bot_three33333";
    const BOT4: &str = "drand_bot_4";
    const BOT5: &str = "drand_bot_5";
    const BOT6: &str = "drand_bot_six_";
    const BOT7: &str = "drand_bot_7";
    const BOT8: &str = "drand_bot_8";

    // register bots
    let msg = nois_drand::msg::ExecuteMsg::RegisterBot {
        moniker: BOT1.to_string(),
    };
    app.execute_contract(Addr::unchecked(BOT1), addr_nois_drand.to_owned(), &msg, &[])
        .unwrap();
    // register bot 2
    let msg = nois_drand::msg::ExecuteMsg::RegisterBot {
        moniker: BOT2.to_string(),
    };
    app.execute_contract(Addr::unchecked(BOT2), addr_nois_drand.to_owned(), &msg, &[])
        .unwrap();
    // register bot 3
    let msg = nois_drand::msg::ExecuteMsg::RegisterBot {
        moniker: BOT3.to_string(),
    };
    app.execute_contract(Addr::unchecked(BOT3), addr_nois_drand.to_owned(), &msg, &[])
        .unwrap();
    // register bot 4
    let msg = nois_drand::msg::ExecuteMsg::RegisterBot {
        moniker: BOT4.to_string(),
    };
    app.execute_contract(Addr::unchecked(BOT4), addr_nois_drand.to_owned(), &msg, &[])
        .unwrap();
    // register bot 5
    let msg = nois_drand::msg::ExecuteMsg::RegisterBot {
        moniker: BOT5.to_string(),
    };
    app.execute_contract(Addr::unchecked(BOT5), addr_nois_drand.to_owned(), &msg, &[])
        .unwrap();
    // register bot 6
    let msg = nois_drand::msg::ExecuteMsg::RegisterBot {
        moniker: BOT6.to_string(),
    };
    app.execute_contract(Addr::unchecked(BOT6), addr_nois_drand.to_owned(), &msg, &[])
        .unwrap();
    // register bot 7
    let msg = nois_drand::msg::ExecuteMsg::RegisterBot {
        moniker: BOT7.to_string(),
    };
    app.execute_contract(Addr::unchecked(BOT7), addr_nois_drand.to_owned(), &msg, &[])
        .unwrap();
    // register bot 8
    let msg = nois_drand::msg::ExecuteMsg::RegisterBot {
        moniker: BOT8.to_string(),
    };
    app.execute_contract(Addr::unchecked(BOT8), addr_nois_drand.to_owned(), &msg, &[])
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
        add: vec![
            BOT1.to_string(),
            BOT2.to_string(),
            BOT3.to_string(),
            BOT4.to_string(),
            BOT5.to_string(),
            BOT6.to_string(),
            BOT7.to_string(),
            BOT8.to_string(),
        ],
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
        // curl -sS https://drand.cloudflare.com/dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493/public/72780
        round: 72780,
        signature: HexBinary::from_hex("86ac005aaffa5e9de34b558c470a111c862e976922e8da34f9dce1a78507dbd53badd554862bc54bd8e44f44ddd8b100").unwrap(),
    };
    let resp = app
        .execute_contract(Addr::unchecked(BOT1), addr_nois_drand.clone(), &msg, &[])
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
        // curl -sS https://drand.cloudflare.com/dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493/public/72780
        round: 72780,
        signature: HexBinary::from_hex("86ac005aaffa5e9de34b558c470a111c862e976922e8da34f9dce1a78507dbd53badd554862bc54bd8e44f44ddd8b100").unwrap(),
    };
    let resp = app
        .execute_contract(Addr::unchecked(BOT2), addr_nois_drand.clone(), &msg, &[])
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
        // curl -sS https://drand.cloudflare.com/dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493/public/72780
        round: 72780,
        signature: HexBinary::from_hex("86ac005aaffa5e9de34b558c470a111c862e976922e8da34f9dce1a78507dbd53badd554862bc54bd8e44f44ddd8b100").unwrap(),
    };
    let resp = app
        .execute_contract(Addr::unchecked(BOT3), addr_nois_drand.clone(), &msg, &[])
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
        // curl -sS https://drand.cloudflare.com/dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493/public/72780
        round: 72780,
        signature: HexBinary::from_hex("86ac005aaffa5e9de34b558c470a111c862e976922e8da34f9dce1a78507dbd53badd554862bc54bd8e44f44ddd8b100").unwrap(),
    };
    let resp = app
        .execute_contract(Addr::unchecked(BOT4), addr_nois_drand.clone(), &msg, &[])
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
        // curl -sS https://drand.cloudflare.com/dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493/public/72780
        round: 72780,
        signature: HexBinary::from_hex("86ac005aaffa5e9de34b558c470a111c862e976922e8da34f9dce1a78507dbd53badd554862bc54bd8e44f44ddd8b100").unwrap(),
    };
    let resp = app
        .execute_contract(Addr::unchecked(BOT5), addr_nois_drand.clone(), &msg, &[])
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
        // curl -sS https://drand.cloudflare.com/dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493/public/72780
        round: 72780,
        signature: HexBinary::from_hex("86ac005aaffa5e9de34b558c470a111c862e976922e8da34f9dce1a78507dbd53badd554862bc54bd8e44f44ddd8b100").unwrap(),
    };
    let resp = app
        .execute_contract(Addr::unchecked(BOT6), addr_nois_drand.clone(), &msg, &[])
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
        // curl -sS https://drand.cloudflare.com/dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493/public/72780
        round: 72780,
        signature: HexBinary::from_hex("86ac005aaffa5e9de34b558c470a111c862e976922e8da34f9dce1a78507dbd53badd554862bc54bd8e44f44ddd8b100").unwrap(),
    };
    let resp = app
        .execute_contract(Addr::unchecked(BOT7), addr_nois_drand.clone(), &msg, &[])
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
        // curl -sS https://drand.cloudflare.com/dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493/public/72780
        round: 72780,
        signature: HexBinary::from_hex("886832ac1b059709a8966347fc447773e15ceff1eada944504fa541ab71c1d1c9ff4f2bbc69f90669a0cf936d018ab52").unwrap(),
    };
    let err = app
        .execute_contract(Addr::unchecked(BOT8), addr_nois_drand, &msg, &[])
        .unwrap_err();

    assert!(matches!(
        err.downcast().unwrap(),
        nois_drand::error::ContractError::SignatureDoesNotMatchState
    ));

    // Check balance nois-gateway
    let balance = query_balance_native(&app, &addr_nois_gateway, "unois").amount;
    assert_eq!(balance, Uint128::new(0));
}
