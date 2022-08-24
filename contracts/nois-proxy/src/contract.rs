#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Deps, DepsMut, Env, IbcMsg, MessageInfo, QueryResponse, Response, StdResult,
};
use nois_ibc_protocol::RequestBeaconPacket;

use crate::ibc::PACKET_LIFETIME;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::TERRAND_CHANNEL;

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
        ExecuteMsg::GetBeacon { round, callback_id } => {
            execute_get_beacon(deps, env, info, round, callback_id)
        }
    }
}

pub fn execute_get_beacon(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    round: u64,
    callback_id: Option<String>,
) -> StdResult<Response> {
    let sender = info.sender.into();
    let packet = RequestBeaconPacket {
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
        .add_attribute("action", "execute_get_beacon");
    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<QueryResponse> {
    match msg {}
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
