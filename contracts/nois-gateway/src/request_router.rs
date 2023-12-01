//! The request router module decides which randomness backend is used

use cosmwasm_std::{
    to_json_binary, Binary, CosmosMsg, DepsMut, Env, HexBinary, IbcMsg, StdAck, StdError,
    StdResult, Timestamp, WasmMsg,
};
use drand_common::{round_after, time_of_round, DRAND_CHAIN_HASH};
use nois_protocol::{InPacketAck, OutPacket, DELIVER_BEACON_PACKET_LIFETIME};

use crate::{
    drand_archive::{archive_lookup, archive_store},
    state::{
        increment_processed_drand_jobs, unprocessed_drand_jobs_dequeue,
        unprocessed_drand_jobs_enqueue, Job, CONFIG,
    },
};

/// The number of jobs that are processed per submission. Use this limit
/// to ensure the gas usage for the submissions is relatively stable.
///
/// Currently a submission without jobs consumes ~500k gas. Every job adds
/// ~45k gas.
const MAX_JOBS_PER_SUBMISSION_WITH_VERIFICATION: u32 = 1;
const MAX_JOBS_PER_SUBMISSION_WITHOUT_VERIFICATION: u32 = 10;

pub struct RoutingReceipt {
    pub queued: bool,
    pub source_id: String,
    pub acknowledgement: StdAck,
    pub msgs: Vec<CosmosMsg>,
}

pub struct NewDrand {
    pub msgs: Vec<CosmosMsg>,
    pub jobs_processed: u32,
}

pub struct RequestRouter {}

impl RequestRouter {
    pub fn new() -> Self {
        Self {}
    }

    pub fn route(
        &self,
        deps: DepsMut,
        env: &Env,
        channel: String,
        after: Timestamp,
        origin: Binary,
    ) -> StdResult<RoutingReceipt> {
        // Here we currently only have one backend
        self.handle_drand_request(deps, env, channel, after, origin)
    }

    fn handle_drand_request(
        &self,
        deps: DepsMut,
        env: &Env,
        channel: String,
        after: Timestamp,
        origin: Binary,
    ) -> StdResult<RoutingReceipt> {
        let (round, source_id) = commit_to_drand_round(after);

        let existing_randomness = archive_lookup(deps.storage, round);

        let job = Job {
            source_id: source_id.clone(),
            channel,
            origin,
        };

        let mut msgs = Vec::<CosmosMsg>::new();

        let queued = if let Some(randomness) = existing_randomness {
            //If the drand round already exists we send it
            increment_processed_drand_jobs(deps.storage, round)?;
            let published = time_of_round(round);
            let msg =
                create_deliver_beacon_ibc_message(env.block.time, job, published, randomness)?;
            msgs.push(msg.into());
            false
        } else {
            unprocessed_drand_jobs_enqueue(deps.storage, round, &job)?;
            let config = CONFIG.load(deps.storage)?;
            if let Some(drand_addr) = config.drand {
                msgs.push(
                    WasmMsg::Execute {
                        contract_addr: drand_addr.into(),
                        msg: to_json_binary(&nois_drand::msg::ExecuteMsg::SetIncentivized {
                            round,
                        })?,
                        funds: vec![],
                    }
                    .into(),
                );
            }
            true
        };

        let acknowledgement = if queued {
            StdAck::success(to_json_binary(&InPacketAck::RequestQueued {
                source_id: source_id.clone(),
            })?)
        } else {
            StdAck::success(to_json_binary(&InPacketAck::RequestProcessed {
                source_id: source_id.clone(),
            })?)
        };

        Ok(RoutingReceipt {
            queued,
            source_id,
            acknowledgement,
            msgs,
        })
    }

    pub fn new_drand(
        &self,
        deps: DepsMut,
        env: Env,
        round: u64,
        randomness: &HexBinary,
        is_verifying_tx: bool,
    ) -> StdResult<NewDrand> {
        archive_store(deps.storage, round, randomness);

        let max_jobs_per_submission = if is_verifying_tx {
            MAX_JOBS_PER_SUBMISSION_WITH_VERIFICATION
        } else {
            MAX_JOBS_PER_SUBMISSION_WITHOUT_VERIFICATION
        };

        let mut msgs = Vec::<CosmosMsg>::new();
        let mut jobs_processed = 0;
        // let max_jobs_per_submission
        while let Some(job) = unprocessed_drand_jobs_dequeue(deps.storage, round)? {
            increment_processed_drand_jobs(deps.storage, round)?;
            let published = time_of_round(round);
            // Use IbcMsg::SendPacket to send packages to the proxies.
            let msg = create_deliver_beacon_ibc_message(
                env.block.time,
                job,
                published,
                randomness.clone(),
            )?;
            msgs.push(msg.into());
            jobs_processed += 1;
            if jobs_processed >= max_jobs_per_submission {
                break;
            }
        }
        Ok(NewDrand {
            msgs,
            jobs_processed,
        })
    }
}

/// Takes the job and turns it into a an IBC message with a `DeliverBeaconPacket`.
fn create_deliver_beacon_ibc_message(
    blocktime: Timestamp,
    job: Job,
    published: Timestamp,
    randomness: HexBinary,
) -> Result<IbcMsg, StdError> {
    let packet = OutPacket::DeliverBeacon {
        randomness,
        published,
        source_id: job.source_id,
        origin: job.origin,
    };
    let msg = IbcMsg::SendPacket {
        channel_id: job.channel,
        data: to_json_binary(&packet)?,
        timeout: blocktime
            .plus_seconds(DELIVER_BEACON_PACKET_LIFETIME)
            .into(),
    };
    Ok(msg)
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
            "drand:dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493:1"
        );

        // Before Drand genesis (https://api3.drand.sh/dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493/info)
        let (round, source) =
            commit_to_drand_round(Timestamp::from_seconds(1677685200).minus_nanos(1));
        assert_eq!(round, 1);
        assert_eq!(
            source,
            "drand:dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493:1"
        );

        // At Drand genesis (https://api3.drand.sh/dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493/info)
        let (round, source) = commit_to_drand_round(Timestamp::from_seconds(1677685200));
        assert_eq!(round, 2);
        assert_eq!(
            source,
            "drand:dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493:2"
        );

        // After Drand genesis
        let (round, _) = commit_to_drand_round(Timestamp::from_seconds(1677685200).plus_nanos(1));
        assert_eq!(round, 2);

        // Drand genesis +26s/27s/28s
        let (round, _) =
            commit_to_drand_round(Timestamp::from_seconds(1677685200).plus_seconds(26));
        assert_eq!(round, 10);
        let (round, _) =
            commit_to_drand_round(Timestamp::from_seconds(1677685200).plus_seconds(27));
        assert_eq!(round, 11);
        let (round, _) =
            commit_to_drand_round(Timestamp::from_seconds(1677685200).plus_seconds(28));
        assert_eq!(round, 11);
    }
}
