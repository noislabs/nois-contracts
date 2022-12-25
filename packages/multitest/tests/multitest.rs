use cosmwasm_std::{
    coin, from_binary, testing::mock_env, to_binary, Addr, BalanceResponse, BankQuery, Coin,
    Decimal, Delegation, HexBinary, Querier, QueryRequest, Uint128, Validator,
};
use cw_multi_test::{App, AppBuilder, ContractWrapper, Executor, StakingInfo};
use nois_multitest::first_attr;

fn query_balance_native(app: &App, address: &Addr, denom: &str) -> Coin {
    let req: QueryRequest<BankQuery> = QueryRequest::Bank(BankQuery::Balance {
        address: address.to_string(),
        denom: denom.to_string(),
    });
    let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
    let balance: BalanceResponse = from_binary(&res).unwrap();

    balance.amount
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

    // Storing nois-icecube code
    let code_nois_icecube = ContractWrapper::new(
        nois_icecube::contract::execute,
        nois_icecube::contract::instantiate,
        nois_icecube::contract::query,
    );
    let code_id_nois_icecube = app.store_code(Box::new(code_nois_icecube));

    //Mint some coins for owner
    mint_native(&mut app, "owner", "unois", 100_000_000);

    // Instantiating nois-icecube contract
    let addr_nois_icecube = app
        .instantiate_contract(
            code_id_nois_icecube,
            Addr::unchecked("owner"),
            &nois_icecube::msg::InstantiateMsg {
                admin_addr: "owner".to_string(),
            },
            &[Coin::new(1_000_000, "unois")],
            "Nois-Icecube",
            None,
        )
        .unwrap();

    //check instantiation and config of nois-icecube contract
    let resp: nois_icecube::msg::ConfigResponse = app
        .wrap()
        .query_wasm_smart(&addr_nois_icecube, &nois_gateway::msg::QueryMsg::Config {})
        .unwrap();
    assert_eq!(
        resp,
        nois_icecube::msg::ConfigResponse {
            admin_addr: Addr::unchecked("owner"),
            drand: None,
        }
    );

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

    // Make the nois-icecube contract aware of the nois-drand contract by
    // setting the drand address in its state
    let msg = nois_icecube::msg::ExecuteMsg::SetDrandAddr {
        addr: addr_nois_gateway.to_string(),
    };
    let resp = app
        .execute_contract(
            Addr::unchecked("a_random_person"),
            addr_nois_icecube.to_owned(),
            &msg,
            &[],
        )
        .unwrap();
    let wasm = resp.events.iter().find(|ev| ev.ty == "wasm").unwrap();
    // Make sure the the tx passed
    assert_eq!(
        first_attr(&wasm.attributes, "nois-drand-address").unwrap(),
        "contract1"
    );

    // Query the new config of nois-icecube containing the nois-drand contract
    let resp: nois_icecube::msg::ConfigResponse = app
        .wrap()
        .query_wasm_smart(&addr_nois_icecube, &nois_gateway::msg::QueryMsg::Config {})
        .unwrap();
    assert_eq!(
        resp,
        nois_icecube::msg::ConfigResponse {
            admin_addr: Addr::unchecked("owner"),
            drand: Option::Some(Addr::unchecked("contract1"))
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
    // Withdraw funds from the icecube contract to the drand contract
    let msg = nois_icecube::msg::ExecuteMsg::SendFundsToDrand {
        funds: coin(300_000, "unois"),
    };
    app.execute_contract(
        Addr::unchecked("an_unhappy_drand_bot_operator"),
        addr_nois_icecube.to_owned(),
        &msg,
        &[],
    )
    .unwrap();
    // Check balance nois-gateway
    let balance = query_balance_native(&app, &addr_nois_gateway, "unois");
    assert_eq!(balance.amount, Uint128::new(300_000));

    // Add round
    let msg = nois_gateway::msg::ExecuteMsg::AddVerifiedRound {
        // curl -sS https://drand.cloudflare.com/public/72785
        round: 72785,
        randomness: HexBinary::from_hex(
            "8b676484b5fb1f37f9ec5c413d7d29883504e5b669f604a1ce68b3388e9ae3d9",
        )
        .unwrap(),
    };
    let _resp = app
        .execute_contract(
            Addr::unchecked("drand_bot"),
            addr_nois_gateway.to_owned(),
            &msg,
            &[],
        )
        .unwrap();

    // Check balance nois-icecube
    let balance = query_balance_native(&app, &addr_nois_icecube, "unois").amount;
    assert_eq!(
        balance,
        Uint128::new(700_000) // 1_000_000(initial_balance) - 300_000(withdrawn) = 700_000
    );
    // Check balance nois-gateway
    let balance = query_balance_native(&app, &addr_nois_gateway, "unois").amount;
    assert_eq!(balance, Uint128::new(300_000));

    // Check balance nois-drand-bot-operator
    // let balance = query_balance_native(&app, &Addr::unchecked("drand_bot"), "unois").amount;
    // assert_eq!(
    //     balance,
    //     Uint128::new(100_000) //incentive
    // );

    // Make nois-icecube delegate
    let msg = nois_icecube::msg::ExecuteMsg::Delegate {
        addr: "noislabs".to_string(),
        amount: Uint128::new(500_000),
    };
    app.execute_contract(
        Addr::unchecked("owner"),
        addr_nois_icecube.to_owned(),
        &msg,
        &[],
    )
    .unwrap();
    // Check balance nois-icecube
    let balance = query_balance_native(&app, &addr_nois_icecube, "unois").amount;
    assert_eq!(
        balance,
        Uint128::new(200_000) // 700_000 - 500_000(staked) = 200_000
    );
    // Check staked amount
    assert_eq!(
        app.wrap()
            .query_all_delegations(&addr_nois_icecube)
            .unwrap()[0],
        Delegation {
            amount: Coin::new(500_000, "unois"),
            delegator: Addr::unchecked("contract0"),
            validator: "noislabs".to_string(),
        }
    );

    //TODO simulte advance many blocks to accumulate some staking rewards

    // Make nois-icecube claim
    let msg = nois_icecube::msg::ExecuteMsg::ClaimRewards {
        addr: "noislabs".to_string(),
    };
    let _err = app
        .execute_contract(Addr::unchecked("owner"), addr_nois_icecube, &msg, &[])
        .unwrap_err();
    //assert_eq!(
    //        nois_icecube::error::ContractError::ContractAlreadySet,
    //        err.downcast().unwrap()
    //
    //    );
    // Check balance nois-icecube
    //let balance = query_balance_native(&app, &addr_nois_icecube, "unois").amount;
    //assert_eq!(
    //    balance,
    //    Uint128::new(800_000) // 900_000 - 100_000(staked) = 800_000
    //);
}
