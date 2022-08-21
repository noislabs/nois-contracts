#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Deps, DepsMut, Env, IbcMsg, MessageInfo, QueryResponse, Response, StdResult, Uint64,
};
use nois_ibc_protocol::PacketMsg;

use crate::ibc::PACKET_LIFETIME;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{GetRoundResponse, LATEST_QUERY_RESULT, TERRAND_CHANNEL};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response> {
    Ok(Response::new().add_attribute("action", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::GetRound { round, callback_id } => {
            execute_get_round(deps, env, info, round, callback_id)
        }
    }
}

pub fn execute_get_round(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    round: Uint64,
    callback_id: Option<String>,
) -> StdResult<Response> {
    let sender = info.sender.into();
    let packet = PacketMsg::GetRound {
        round,
        sender,
        callback_id,
    };
    let channel_id = TERRAND_CHANNEL.load(deps.storage)?;
    let msg = IbcMsg::SendPacket {
        channel_id,
        data: to_binary(&packet)?,
        timeout: env.block.time.plus_seconds(PACKET_LIFETIME).into(),
    };

    let res = Response::new()
        .add_message(msg)
        .add_attribute("action", "handle_check_remote_balance");
    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<QueryResponse> {
    match msg {
        QueryMsg::LatestGetRoundResult {} => to_binary(&query_latest_get_round_result(deps)?),
    }
}

fn query_latest_get_round_result(deps: Deps) -> StdResult<GetRoundResponse> {
    let results = LATEST_QUERY_RESULT.load(deps.storage)?;
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    const CREATOR: &str = "creator";

    #[test]
    fn instantiate_works() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {};
        let info = mock_info(CREATOR, &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }
}
