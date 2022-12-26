use cosmwasm_std::{testing::mock_env, Addr, Coin, Decimal, HexBinary, Validator};
use cw_multi_test::{App, AppBuilder, ContractWrapper, Executor, StakingInfo};

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
        .execute_contract(Addr::unchecked("drand_bot"), addr_nois_gateway, &msg, &[])
        .unwrap();
}
