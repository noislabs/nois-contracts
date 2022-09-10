use cosmwasm_schema::write_api;

use nois_proxy::msg::{ExecuteMsg, InstantiateMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        // Currently broken (https://github.com/CosmWasm/cosmwasm/issues/1411)
        // query: QueryMsg,
        execute: ExecuteMsg,
    }
}
