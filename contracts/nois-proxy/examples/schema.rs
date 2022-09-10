use cosmwasm_schema::write_api;

use nois::proxy::ExecuteMsg;
use nois_proxy::msg::InstantiateMsg;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        // Currently broken (https://github.com/CosmWasm/cosmwasm/issues/1411)
        // query: QueryMsg,
        execute: ExecuteMsg,
    }
}
