mod drand_jobs1;
mod drand_jobs2;

use cosmwasm_std::{StdResult, Storage};
pub use drand_jobs1::{Job, DRAND_JOBS_V1_END, DRAND_JOBS_V1_START};

// This is too inefficient for drand_jobs1 and only used in an informational query
pub use drand_jobs2::all_unprocessed_drand_jobs;

// New jobs always go to v2
pub use drand_jobs2::unprocessed_drand_jobs_enqueue;

/// Gets the number of unprocessed drand jobs queue of this round.
/// This is inefficient for many jobs in a single round.
pub fn unprocessed_drand_jobs_len(storage: &dyn Storage, round: u64) -> StdResult<u32> {
    let l1 = drand_jobs1::unprocessed_drand_jobs_len(storage, round)?;
    let l2 = drand_jobs2::unprocessed_drand_jobs_len(storage, round)?;
    Ok(l1 + l2)
}

pub fn unprocessed_drand_jobs_dequeue(
    storage: &mut dyn Storage,
    round: u64,
) -> StdResult<Option<Job>> {
    if let Some(job_v1) = drand_jobs1::unprocessed_drand_jobs_dequeue(storage, round)? {
        return Ok(Some(job_v1));
    }
    if let Some(job_v2) = drand_jobs2::unprocessed_drand_jobs_dequeue(storage, round)? {
        return Ok(Some(job_v2));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{testing::MockStorage, Binary};

    use super::*;

    fn make_job(id: u32) -> Job {
        Job {
            channel: "chan-123".to_string(),
            source_id: "drannd:foo:bar".to_string(),
            origin: Binary::from(id.to_be_bytes()),
        }
    }

    #[test]
    fn unprocessed_drand_jobs_dequeue_works() {
        let mut storage = MockStorage::default();

        let round = 3;

        let job = unprocessed_drand_jobs_dequeue(&mut storage, round).unwrap();
        assert_eq!(job, None);

        let job1 = make_job(1);
        let job2 = make_job(2);
        let job3 = make_job(3);
        let job4 = make_job(4);

        drand_jobs1::unprocessed_drand_jobs_enqueue(&mut storage, round, &job1).unwrap();
        drand_jobs1::unprocessed_drand_jobs_enqueue(&mut storage, round, &job2).unwrap();
        drand_jobs2::unprocessed_drand_jobs_enqueue(&mut storage, round, &job3).unwrap();
        drand_jobs2::unprocessed_drand_jobs_enqueue(&mut storage, round, &job4).unwrap();

        let job = unprocessed_drand_jobs_dequeue(&mut storage, round).unwrap();
        assert_eq!(job, Some(job1));
        let job = unprocessed_drand_jobs_dequeue(&mut storage, round).unwrap();
        assert_eq!(job, Some(job2));
        let job = unprocessed_drand_jobs_dequeue(&mut storage, round).unwrap();
        assert_eq!(job, Some(job3));
        let job = unprocessed_drand_jobs_dequeue(&mut storage, round).unwrap();
        assert_eq!(job, Some(job4));
        let job = unprocessed_drand_jobs_dequeue(&mut storage, round).unwrap();
        assert_eq!(job, None);
    }

    #[test]
    fn unprocessed_drand_jobs_len_works() {
        let mut storage = MockStorage::default();

        let round = 3;

        let job1 = make_job(1);
        let job2 = make_job(2);
        let job3 = make_job(3);
        let job4 = make_job(4);

        assert_eq!(unprocessed_drand_jobs_len(&storage, round).unwrap(), 0);
        drand_jobs1::unprocessed_drand_jobs_enqueue(&mut storage, round, &job1).unwrap();
        assert_eq!(unprocessed_drand_jobs_len(&storage, round).unwrap(), 1);
        drand_jobs1::unprocessed_drand_jobs_enqueue(&mut storage, round, &job2).unwrap();
        assert_eq!(unprocessed_drand_jobs_len(&storage, round).unwrap(), 2);
        drand_jobs2::unprocessed_drand_jobs_enqueue(&mut storage, round, &job3).unwrap();
        assert_eq!(unprocessed_drand_jobs_len(&storage, round).unwrap(), 3);
        drand_jobs2::unprocessed_drand_jobs_enqueue(&mut storage, round, &job4).unwrap();
        assert_eq!(unprocessed_drand_jobs_len(&storage, round).unwrap(), 4);

        let _ = unprocessed_drand_jobs_dequeue(&mut storage, round).unwrap();
        assert_eq!(unprocessed_drand_jobs_len(&storage, round).unwrap(), 3);
        let _ = unprocessed_drand_jobs_dequeue(&mut storage, round).unwrap();
        assert_eq!(unprocessed_drand_jobs_len(&storage, round).unwrap(), 2);
        let _ = unprocessed_drand_jobs_dequeue(&mut storage, round).unwrap();
        assert_eq!(unprocessed_drand_jobs_len(&storage, round).unwrap(), 1);
        let _ = unprocessed_drand_jobs_dequeue(&mut storage, round).unwrap();
        assert_eq!(unprocessed_drand_jobs_len(&storage, round).unwrap(), 0);
        let _ = unprocessed_drand_jobs_dequeue(&mut storage, round).unwrap();
        assert_eq!(unprocessed_drand_jobs_len(&storage, round).unwrap(), 0);
    }
}
