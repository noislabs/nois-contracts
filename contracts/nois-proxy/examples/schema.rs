use cosmwasm_schema::write_api;

use nois_proxy::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, SudoMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        query: QueryMsg,
        execute: ExecuteMsg,
        sudo: SudoMsg,
    }
}
