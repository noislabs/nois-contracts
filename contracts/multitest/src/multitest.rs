mod tests {

    use cosmwasm_std::{Addr, Uint128, Coin};
    use cw_multi_test::{App, ContractWrapper, Executor};

    #[test]
    fn integration_test() {
        // Insantiate a chain mock environment
        let mut app = App::default();
        // Storing nois-delegator code
        let code_nois_delegator = ContractWrapper::new(
            nois_delegator::contract::execute,
            nois_delegator::contract::instantiate,
            nois_delegator::contract::query,
        );
        let code_id_nois_delegator = app.store_code(Box::new(code_nois_delegator));

        // Instantiating nois-delegator contract
        let addr_nois_delegator = app
            .instantiate_contract(
                code_id_nois_delegator,
                Addr::unchecked("owner"),
                &nois_delegator::msg::InstantiateMsg {
                    admin_addr: "owner".to_string(),
                },
                &[],
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
                    delegator_contract: addr_nois_delegator.to_string(),
                    incentive_amount: Uint128::new(1_000_000),
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
                incentive_amount: Uint128::new(1_000_000),
                incentive_denom: "unois".to_string(),
                delegator_contract: Addr::unchecked("contract0"),
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
        let wasm = resp.events.iter().find(|ev|ev.ty == "wasm").unwrap();
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
        
        // Instantiating nois-oracle contract
        let addr_nois_proxy = app
            .instantiate_contract(
                code_id_nois_proxy,
                Addr::unchecked("owner"),
                &nois_proxy::msg::InstantiateMsg {
                    prices: vec![Coin::new(1_000000, "unoisx")],
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
                config: nois_proxy::state::Config{
                    prices: vec![Coin::new(1_000000, "unoisx")],
                    withdrawal_address: Addr::unchecked("dao_dao_dao_dao_dao"),
                    test_mode: false,
                }, 
                   
            }
        );
    }

}
