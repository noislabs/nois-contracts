use cosmwasm_std::{testing::mock_env, Addr, Coin, Decimal, HexBinary, Validator};
use cw_multi_test::{AppBuilder, ContractWrapper, Executor, StakingInfo};
use nois_multitest::mint_native;

const PAYMENT: u64 = 17;
const SINK: &str = "sink";
const COMMUNOTY_POOL: &str = "community_pool";

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
            &nois_gateway::msg::InstantiateMsg {
                manager: "manager".to_string(),
                price: Coin::new(1, "unois"),
                payment_code_id: PAYMENT,
                sink: SINK.to_string(),
                community_pool: COMMUNOTY_POOL.to_string(),
            },
            &[],
            "Nois-Gateway",
            None,
        )
        .unwrap();

    // Check initial config
    let resp: nois_gateway::msg::ConfigResponse = app
        .wrap()
        .query_wasm_smart(&addr_nois_gateway, &nois_gateway::msg::QueryMsg::Config {})
        .unwrap();
    assert_eq!(
        resp,
        nois_gateway::msg::ConfigResponse {
            drand: None,
            manager: Addr::unchecked("manager"),
            price: Coin::new(1, "unois"),
            payment_code_id: PAYMENT,
            sink: Addr::unchecked(SINK),
            community_pool: Addr::unchecked(COMMUNOTY_POOL),
        }
    );

    const DRAND: &str = "drand_verifier_7";

    // Set drand
    let msg = nois_gateway::msg::ExecuteMsg::SetConfig {
        manager: None,
        price: None,
        drand_addr: Some(DRAND.to_string()),
        payment_code_id: None,
    };
    let _resp = app
        .execute_contract(
            Addr::unchecked("manager"),
            addr_nois_gateway.clone(),
            &msg,
            &[],
        )
        .unwrap();

    // Check updated config
    let resp: nois_gateway::msg::ConfigResponse = app
        .wrap()
        .query_wasm_smart(&addr_nois_gateway, &nois_gateway::msg::QueryMsg::Config {})
        .unwrap();
    assert_eq!(
        resp,
        nois_gateway::msg::ConfigResponse {
            drand: Some(Addr::unchecked(DRAND)),
            manager: Addr::unchecked("manager"),
            price: Coin::new(1, "unois"),
            payment_code_id: PAYMENT,
            sink: Addr::unchecked(SINK),
            community_pool: Addr::unchecked(COMMUNOTY_POOL),
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
                callback_gas_limit: 500_000,
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
                callback_gas_limit: 500_000,
                payment: None,
            },
        }
    );

    // Add verified round
    let msg = nois_gateway::msg::ExecuteMsg::AddVerifiedRound {
        // curl -sS https://drand.cloudflare.com/dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493/public/72785
        round: 72785,
        randomness: HexBinary::from_hex(
            "650be14f6ffd7dcb67df9138c3b7d7d6bca455d0438fc81d3fbb24a4ee038f36",
        )
        .unwrap(),
        is_verifying_tx: true,
    };
    let _resp = app
        .execute_contract(Addr::unchecked(DRAND), addr_nois_gateway, &msg, &[])
        .unwrap();
}
