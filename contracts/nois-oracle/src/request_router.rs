//! The request router module decides which randomness backend is used

use cosmwasm_std::{CosmosMsg, DepsMut, Env, StdResult, Timestamp};
use nois_protocol::{RequestBeaconPacketAck, StdAck};

use crate::{
    contract::create_deliver_beacon_ibc_message,
    drand::{round_after, DRAND_CHAIN_HASH},
    state::{increment_processed_jobs, unprocessed_jobs_enqueue, Job, BEACONS},
};

pub struct RoutingReceipt {
    pub acknowledgement: StdAck,
    pub msgs: Vec<CosmosMsg>,
}

pub struct RequestRouter {}

impl RequestRouter {
    pub fn route(
        &self,
        deps: DepsMut,
        env: Env,
        channel: String,
        after: Timestamp,
        sender: String,
        job_id: String,
    ) -> StdResult<RoutingReceipt> {
        let (round, source_id) = commit_to_drand_round(after);

        // Does round exist already?

        // Implementation using query
        // let BeaconResponse { beacon } = deps
        //     .querier
        //     .query_wasm_smart(&self.drand_addr, &QueryMsg::Beacon { round })?;
        // let randomness = beacon.map(|b| b.randomness);

        // Implementation using storage
        let randomness = BEACONS.may_load(deps.storage, round)?.map(|b| b.randomness);

        let job = Job {
            source_id: source_id.clone(),
            channel,
            sender,
            job_id,
        };

        let mut msgs = Vec::<CosmosMsg>::new();

        let acknowledgement = if let Some(randomness) = randomness {
            //If the drand round already exists we send it
            increment_processed_jobs(deps.storage, round)?;
            let msg = create_deliver_beacon_ibc_message(env.block.time, job, randomness)?;
            msgs.push(msg.into());
            StdAck::success(&RequestBeaconPacketAck::Processed { source_id })
        } else {
            unprocessed_jobs_enqueue(deps.storage, round, &job)?;
            StdAck::success(&RequestBeaconPacketAck::Queued { source_id })
        };

        Ok(RoutingReceipt {
            acknowledgement,
            msgs,
        })
    }
}

/// Calculates the next round in the future, i.e. publish time > base time.
fn commit_to_drand_round(after: Timestamp) -> (u64, String) {
    let round = round_after(after);
    let source_id = format!("drand:{}:{}", DRAND_CHAIN_HASH, round);
    (round, source_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commit_to_drand_round_works() {
        // UNIX epoch
        let (round, source) = commit_to_drand_round(Timestamp::from_seconds(0));
        assert_eq!(round, 1);
        assert_eq!(
            source,
            "drand:8990e7a9aaed2ffed73dbd7092123d6f289930540d7651336225dc172e51b2ce:1"
        );

        // Before Drand genesis (https://api3.drand.sh/info)
        let (round, source) =
            commit_to_drand_round(Timestamp::from_seconds(1595431050).minus_nanos(1));
        assert_eq!(round, 1);
        assert_eq!(
            source,
            "drand:8990e7a9aaed2ffed73dbd7092123d6f289930540d7651336225dc172e51b2ce:1"
        );

        // At Drand genesis (https://api3.drand.sh/info)
        let (round, source) = commit_to_drand_round(Timestamp::from_seconds(1595431050));
        assert_eq!(round, 2);
        assert_eq!(
            source,
            "drand:8990e7a9aaed2ffed73dbd7092123d6f289930540d7651336225dc172e51b2ce:2"
        );

        // After Drand genesis (https://api3.drand.sh/info)
        let (round, _) = commit_to_drand_round(Timestamp::from_seconds(1595431050).plus_nanos(1));
        assert_eq!(round, 2);

        // Drand genesis +29s/30s/31s
        let (round, _) =
            commit_to_drand_round(Timestamp::from_seconds(1595431050).plus_seconds(29));
        assert_eq!(round, 2);
        let (round, _) =
            commit_to_drand_round(Timestamp::from_seconds(1595431050).plus_seconds(30));
        assert_eq!(round, 3);
        let (round, _) =
            commit_to_drand_round(Timestamp::from_seconds(1595431050).plus_seconds(31));
        assert_eq!(round, 3);
    }
}
