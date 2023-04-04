// Testing utils. See tests folder for actual tests.

use cosmwasm_std::{
    coin, from_binary, to_binary, Addr, Attribute, BalanceResponse, BankQuery, Coin, Querier,
    QueryRequest,
};
use cw_multi_test::App;

/// Gets the value of the first attribute with the given key
pub fn first_attr(data: impl AsRef<[Attribute]>, search_key: &str) -> Option<String> {
    data.as_ref().iter().find_map(|a| {
        if a.key == search_key {
            Some(a.value.clone())
        } else {
            None
        }
    })
}

pub fn query_balance_native(app: &App, address: &Addr, denom: &str) -> Coin {
    let req: QueryRequest<BankQuery> = QueryRequest::Bank(BankQuery::Balance {
        address: address.to_string(),
        denom: denom.to_string(),
    });
    let res = app.raw_query(&to_binary(&req).unwrap()).unwrap().unwrap();
    let balance: BalanceResponse = from_binary(&res).unwrap();

    balance.amount
}

pub fn mint_native(
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

pub fn payment_initial() -> Option<Coin> {
    Some(coin(2_000000, "unois"))
}
