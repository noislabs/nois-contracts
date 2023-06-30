//! The request router module decides which randomness backend is used

use cosmwasm_std::{
    to_binary, Binary, CosmosMsg, DepsMut, Env, HexBinary, IbcMsg, StdError, StdResult, Timestamp,
};
use drand_common::{time_of_round, valid_round_after, DRAND_CHAIN_HASH};
use nois_protocol::{InPacketAck, OutPacket, StdAck, DELIVER_BEACON_PACKET_LIFETIME};

use crate::{
    drand_archive::{archive_lookup, archive_store},
    state::{drand_jobs2, increment_processed_drand_jobs, Job},
};

/// The number of jobs that are processed per submission. Use this limit
/// to ensure the gas usage for the submissions is relatively stable.
///
/// Currently a submission without jobs consumes ~600k gas. Every job adds
/// ~50k gas.
const MAX_JOBS_PER_SUBMISSION_WITH_VERIFICATION: u32 = 2;
const MAX_JOBS_PER_SUBMISSION_WITHOUT_VERIFICATION: u32 = 14;

pub struct RoutingReceipt {
    pub queued: bool,
    pub source_id: String,
    pub acknowledgement: StdAck,
    pub msgs: Vec<CosmosMsg>,
}

pub struct NewDrand {
    pub msgs: Vec<CosmosMsg>,
    pub jobs_processed: u32,
    pub jobs_left: u32,
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
        self.handle_drand(deps, env, channel, after, origin)
    }

    fn handle_drand(
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
            drand_jobs2::unprocessed_drand_jobs_enqueue(deps.storage, round, &job)?;
            true
        };

        let acknowledgement = if queued {
            StdAck::success(InPacketAck::RequestQueued {
                source_id: source_id.clone(),
            })
        } else {
            StdAck::success(InPacketAck::RequestProcessed {
                source_id: source_id.clone(),
            })
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
        while let Some(job) = drand_jobs2::unprocessed_drand_jobs_dequeue(deps.storage, round)? {
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
        let jobs_left = drand_jobs2::unprocessed_drand_jobs_len(deps.storage, round)?;
        Ok(NewDrand {
            msgs,
            jobs_processed,
            jobs_left,
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
        data: to_binary(&packet)?,
        timeout: blocktime
            .plus_seconds(DELIVER_BEACON_PACKET_LIFETIME)
            .into(),
    };
    Ok(msg)
}

/// Calculates the next round in the future, i.e. publish time > base time.
fn commit_to_drand_round(after: Timestamp) -> (u64, String) {
    let round = valid_round_after(after);
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
        assert_eq!(round, 10);
        assert_eq!(
            source,
            "drand:dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493:10"
        );

        // Before Drand genesis (https://api3.drand.sh/dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493/info)
        let (round, source) =
            commit_to_drand_round(Timestamp::from_seconds(1677685200).minus_nanos(1));
        assert_eq!(round, 10);
        assert_eq!(
            source,
            "drand:dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493:10"
        );

        // At Drand genesis (https://api3.drand.sh/dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493/info)
        let (round, source) = commit_to_drand_round(Timestamp::from_seconds(1677685200));
        assert_eq!(round, 10);
        assert_eq!(
            source,
            "drand:dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493:10"
        );

        // After Drand genesis
        let (round, _) = commit_to_drand_round(Timestamp::from_seconds(1677685200).plus_nanos(1));
        assert_eq!(round, 10);

        // Drand genesis +29s/30s/31s
        let (round, _) =
            commit_to_drand_round(Timestamp::from_seconds(1677685200).plus_seconds(26));
        assert_eq!(round, 10);
        let (round, _) =
            commit_to_drand_round(Timestamp::from_seconds(1677685200).plus_seconds(27));
        assert_eq!(round, 20);
        let (round, _) =
            commit_to_drand_round(Timestamp::from_seconds(1677685200).plus_seconds(28));
        assert_eq!(round, 20);
    }
}
