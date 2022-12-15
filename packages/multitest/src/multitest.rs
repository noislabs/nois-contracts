mod tests {

    use cosmwasm_std::{
        testing::mock_env, Addr, Coin, Decimal, Delegation, HexBinary, Querier, Uint128, Validator,
    };
    use cw_multi_test::{App, AppBuilder, ContractWrapper, Executor, StakingInfo};

    use cosmwasm_std::{from_binary, to_binary, BalanceResponse, BankQuery, QueryRequest};

    fn query_balance_native(app: &App, address: &Addr, denom: &str) -> Coin {
        let req: QueryRequest<BankQuery> = QueryRequest::Bank(BankQuery::Balance {
            address: address.to_string(),
            denom: denom.to_string(),
        });
        let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
        let balance: BalanceResponse = from_binary(&res).unwrap();

        balance.amount
    }
    fn mint_native(app: &mut App, beneficiary: String, denom: String, amount: u128) {
        app.sudo(cw_multi_test::SudoMsg::Bank(
            cw_multi_test::BankSudo::Mint {
                to_address: beneficiary,
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

        // Storing nois-delegator code
        let code_nois_delegator = ContractWrapper::new(
            nois_delegator::contract::execute,
            nois_delegator::contract::instantiate,
            nois_delegator::contract::query,
        );
        let code_id_nois_delegator = app.store_code(Box::new(code_nois_delegator));
        //Mint some coins for owner
        mint_native(
            &mut app,
            "owner".to_string(),
            "unois".to_string(),
            100_000_000,
        );

        // Instantiating nois-delegator contract
        let addr_nois_delegator = app
            .instantiate_contract(
                code_id_nois_delegator,
                Addr::unchecked("owner"),
                &nois_delegator::msg::InstantiateMsg {
                    admin_addr: "owner".to_string(),
                },
                &[Coin::new(1_000_000, "unois")],
                "Nois-Delegator",
                None,
            )
            .unwrap();

        //check instantiation and config of nois-delegator contract
        let resp: nois_delegator::msg::ConfigResponse = app
            .wrap()
            .query_wasm_smart(&addr_nois_delegator, &nois_oracle::msg::QueryMsg::Config {})
            .unwrap();
        assert_eq!(
            resp,
            nois_delegator::msg::ConfigResponse {
                admin_addr: "owner".to_string(),
                nois_oracle_contract_addr: Option::None
            }
        );

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
                &nois_oracle::msg::InstantiateMsg {
                    incentive_amount: Uint128::new(100_000),
                    incentive_denom: "unois".to_string(),
                    min_round: 0,
                },
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

        assert_eq!(
            resp,
            nois_oracle::msg::ConfigResponse {
                min_round: 0,
                incentive_amount: Uint128::new(100_000),
                incentive_denom: "unois".to_string(),
            }
        );
        // Make the nois-delegator contract aware of the nois-oracle contract by setting the oracle address in its state
        let msg = nois_delegator::msg::ExecuteMsg::SetNoisOracleContractAddr {
            addr: addr_nois_oracle.to_string(),
        };
        let resp = app
            .execute_contract(
                Addr::unchecked("a_random_person"),
                addr_nois_delegator.to_owned(),
                &msg,
                &[],
            )
            .unwrap();
        let wasm = resp.events.iter().find(|ev| ev.ty == "wasm").unwrap();
        // Make sure the the tx passed
        assert_eq!(
            wasm.attributes
                .iter()
                .find(|attr| attr.key == "nois-oracle-address")
                .unwrap()
                .value,
            "contract1"
        );
        //Query the new config of nois-delegator containing the nois-oracle contract
        let resp: nois_delegator::msg::ConfigResponse = app
            .wrap()
            .query_wasm_smart(&addr_nois_delegator, &nois_oracle::msg::QueryMsg::Config {})
            .unwrap();
        assert_eq!(
            resp,
            nois_delegator::msg::ConfigResponse {
                admin_addr: "owner".to_string(),
                nois_oracle_contract_addr: Option::Some(Addr::unchecked("contract1"))
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
        // Withdraw funds from the delegator contract to the oracle contract
        let msg = nois_delegator::msg::ExecuteMsg::SendFundsToOracle {
            amount: (Uint128::new(300_000)),
        };
        app.execute_contract(
            Addr::unchecked("an_unhappy_drand_bot_operator"),
            addr_nois_delegator.to_owned(),
            &msg,
            &[],
        )
        .unwrap();
        // Check balance nois-oracle
        let balance = query_balance_native(&app, &addr_nois_oracle, "unois").amount;
        assert_eq!(balance, Uint128::new(300_000));

        // register bot
        let msg = nois_oracle::msg::ExecuteMsg::RegisterBot {
            moniker: "drand_bot".to_string(),
        };
        app.execute_contract(
            Addr::unchecked("drand_bot"),
            addr_nois_oracle.to_owned(),
            &msg,
            &[],
        )
        .unwrap();
        // Add round
        let msg = nois_oracle::msg::ExecuteMsg::AddRound {
            // curl -sS https://drand.cloudflare.com/public/72785
            round: 72785,
            previous_signature: HexBinary::from_hex("a609e19a03c2fcc559e8dae14900aaefe517cb55c840f6e69bc8e4f66c8d18e8a609685d9917efbfb0c37f058c2de88f13d297c7e19e0ab24813079efe57a182554ff054c7638153f9b26a60e7111f71a0ff63d9571704905d3ca6df0b031747").unwrap(),
            signature: HexBinary::from_hex("82f5d3d2de4db19d40a6980e8aa37842a0e55d1df06bd68bddc8d60002e8e959eb9cfa368b3c1b77d18f02a54fe047b80f0989315f83b12a74fd8679c4f12aae86eaf6ab5690b34f1fddd50ee3cc6f6cdf59e95526d5a5d82aaa84fa6f181e42").unwrap(),
        };
        let resp = app
            .execute_contract(
                Addr::unchecked("drand_bot"),
                addr_nois_oracle.to_owned(),
                &msg,
                &[],
            )
            .unwrap();

        let wasm = resp.events.iter().find(|ev| ev.ty == "wasm").unwrap();
        // Make sure that there is an incentive for the registered bot
        assert_eq!(
            wasm.attributes
                .iter()
                .find(|attr| attr.key == "bot_incentive")
                .unwrap()
                .value,
            "100000unois"
        );
        // Check balance nois-delegator
        let balance = query_balance_native(&app, &addr_nois_delegator, "unois").amount;
        assert_eq!(
            balance,
            Uint128::new(700_000) // 1_000_000(initial_balance) - 300_000(withdrawn) = 700_000
        );
        // Check balance nois-oracle
        let balance = query_balance_native(&app, &addr_nois_oracle, "unois").amount;
        assert_eq!(balance, Uint128::new(200_000));
        // Check balance nois-drand-bot-operator
        let balance = query_balance_native(&app, &Addr::unchecked("drand_bot"), "unois").amount;
        assert_eq!(
            balance,
            Uint128::new(100_000) //incentive
        );

        // Make nois-delegator delegate
        let msg = nois_delegator::msg::ExecuteMsg::Delegate {
            addr: "noislabs".to_string(),
            amount: Uint128::new(500_000),
        };
        app.execute_contract(
            Addr::unchecked("owner"),
            addr_nois_delegator.to_owned(),
            &msg,
            &[],
        )
        .unwrap();
        // Check balance nois-delegator
        let balance = query_balance_native(&app, &addr_nois_delegator, "unois").amount;
        assert_eq!(
            balance,
            Uint128::new(200_000) // 700_000 - 500_000(staked) = 200_000
        );
        // Check staked amount
        assert_eq!(
            app.wrap()
                .query_all_delegations(&addr_nois_delegator)
                .unwrap()[0],
            Delegation {
                amount: Coin::new(500_000, "unois"),
                delegator: Addr::unchecked("contract0"),
                validator: "noislabs".to_string(),
            }
        );

        //TODO simulte advance many blocks to accumulate some staking rewards

        // Make nois-delegator claim
        let msg = nois_delegator::msg::ExecuteMsg::ClaimRewards {
            addr: "noislabs".to_string(),
        };
        let _err = app
            .execute_contract(Addr::unchecked("owner"), addr_nois_delegator, &msg, &[])
            .unwrap_err();
        //assert_eq!(
        //        nois_delegator::error::ContractError::ContractAlreadySet,
        //        err.downcast().unwrap()
        //
        //    );
        // Check balance nois-delegator
        //let balance = query_balance_native(&app, &addr_nois_delegator, "unois").amount;
        //assert_eq!(
        //    balance,
        //    Uint128::new(800_000) // 900_000 - 100_000(staked) = 800_000
        //);
    }
}
