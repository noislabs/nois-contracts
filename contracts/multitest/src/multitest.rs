#[cfg(test)]
mod tests {
    use cosmwasm_std::{Addr, Uint128};
    use cw_multi_test::{App, ContractWrapper, Executor};
    use nois_oracle::msg::{ConfigResponse, InstantiateMsg};

    #[test]
    fn instantiation_works() {
        let mut app = App::default();
        let code_nois_delegator = ContractWrapper::new(
            nois_delegator::contract::execute,
            nois_delegator::contract::instantiate,
            nois_delegator::contract::query,
        );
        let code_id_nois_delegator = app.store_code(Box::new(code_nois_delegator));
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

        let code_nois_oracle = ContractWrapper::new(
            nois_oracle::contract::execute,
            nois_oracle::contract::instantiate,
            nois_oracle::contract::query,
        );
        let code_id_nois_oracle = app.store_code(Box::new(code_nois_oracle));
        let addr = app
            .instantiate_contract(
                code_id_nois_oracle,
                Addr::unchecked("owner"),
                &InstantiateMsg {
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
        let resp: ConfigResponse = app
            .wrap()
            .query_wasm_smart(addr, &nois_oracle::msg::QueryMsg::Config {})
            .unwrap();

        assert_eq!(
            resp,
            nois_oracle::msg::ConfigResponse {
                min_round: 0,
                incentive_amount: Uint128::new(1_000_000),
                incentive_denom: "unois".to_string(),
                delegator_contract: Addr::unchecked("contract0"),
            }
        );
    }
}
