use cosmwasm_std::{Order, StdResult, Storage};
use cw_storage_plus::Map;

use super::Job;

/// A map from (round, job ID) here job ID is a round specific auto incrementing ID
const JOBS: Map<(u32, u16), Job> = Map::new("djobs");
const LAST_JOB_ID: Map<u32, u16> = Map::new("djids");

/// Add an element to the unprocessed drand jobs queue of this round
pub fn unprocessed_drand_jobs_enqueue(
    storage: &mut dyn Storage,
    round: u64,
    value: &Job,
) -> StdResult<()> {
    let round: u32 = round.try_into().expect("round must not exceed u32 range");
    let new_id = LAST_JOB_ID.may_load(storage, round)?.unwrap_or_default() + 1;
    JOBS.save(storage, (round, new_id), value)?;
    LAST_JOB_ID.save(storage, round, &new_id)?;
    Ok(())
}

/// Remove an element from the unprocessed drand jobs queue of this round
pub fn unprocessed_drand_jobs_dequeue(
    storage: &mut dyn Storage,
    round: u64,
) -> StdResult<Option<Job>> {
    let round: u32 = round.try_into().expect("round must not exceed u32 range");
    let first = JOBS
        .prefix(round)
        .range(storage, None, None, Order::Ascending)
        .next();
    let Some(found) = first else {
        return Ok(None);
    };
    let (id, job) = found?;
    JOBS.remove(storage, (round, id));
    Ok(Some(job))
}

/// Gets the number of unprocessed drand jobs queue of this round.
/// This is inefficient for many jobs in a single round.
pub fn unprocessed_drand_jobs_len(storage: &dyn Storage, round: u64) -> StdResult<u32> {
    let round: u32 = round.try_into().expect("round must not exceed u32 range");
    let count = JOBS
        .prefix(round)
        .keys_raw(storage, None, None, Order::Ascending)
        .count();
    Ok(count as u32)
}

pub fn all_unprocessed_drand_jobs(
    storage: &dyn Storage,
    order: Order,
    offset: usize,
    limit: usize,
) -> StdResult<Vec<Job>> {
    JOBS.range_raw(storage, None, None, order)
        .skip(offset)
        .take(limit)
        .map(|res| res.map(|ok| ok.1))
        .collect::<StdResult<Vec<_>>>()
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{testing::MockStorage, Binary};

    use super::*;

    #[test]
    fn all_unprocessed_drand_jobs_works() {
        let mut storage = MockStorage::default();

        let jobs = all_unprocessed_drand_jobs(&storage, Order::Ascending, 0, 100).unwrap();
        assert_eq!(jobs, []);

        let job1 = Job {
            channel: "chan-123".to_string(),
            source_id: "drannd:foo:bar".to_string(),
            origin: Binary::from([1, 2, 1, 2]),
        };
        unprocessed_drand_jobs_enqueue(&mut storage, 3, &job1).unwrap();

        let jobs = all_unprocessed_drand_jobs(&storage, Order::Ascending, 0, 100).unwrap();
        assert_eq!(jobs, &[job1.clone()]);

        let job2 = Job {
            channel: "chan-123".to_string(),
            source_id: "drannd:foo:baz".to_string(),
            origin: Binary::from([17, 4]),
        };
        unprocessed_drand_jobs_enqueue(&mut storage, 3, &job2).unwrap();

        let jobs = all_unprocessed_drand_jobs(&storage, Order::Ascending, 0, 100).unwrap();
        assert_eq!(jobs, &[job1.clone(), job2.clone()]);
        let jobs = all_unprocessed_drand_jobs(&storage, Order::Ascending, 0, 1).unwrap();
        assert_eq!(jobs, &[job1.clone()]);
        let jobs = all_unprocessed_drand_jobs(&storage, Order::Ascending, 1, 100).unwrap();
        assert_eq!(jobs, &[job2.clone()]);
        let jobs = all_unprocessed_drand_jobs(&storage, Order::Descending, 0, 100).unwrap();
        assert_eq!(jobs, &[job2.clone(), job1.clone()]);
        let jobs = all_unprocessed_drand_jobs(&storage, Order::Descending, 0, 1).unwrap();
        assert_eq!(jobs, &[job2.clone()]);
        let jobs = all_unprocessed_drand_jobs(&storage, Order::Descending, 1, 100).unwrap();
        assert_eq!(jobs, &[job1.clone()]);

        let _ = unprocessed_drand_jobs_dequeue(&mut storage, 3).unwrap();

        let jobs = all_unprocessed_drand_jobs(&storage, Order::Ascending, 0, 100).unwrap();
        assert_eq!(jobs, &[job2.clone()]);

        // new job in higher round
        let job3 = Job {
            channel: "chan-123".to_string(),
            source_id: "drannd:foo:test".to_string(),
            origin: Binary::from([42, 42]),
        };
        unprocessed_drand_jobs_enqueue(&mut storage, 4, &job3).unwrap();

        // new job in lower round
        let job4 = Job {
            channel: "chan-123".to_string(),
            source_id: "drannd:foo:test".to_string(),
            origin: Binary::from([12, 21]),
        };
        unprocessed_drand_jobs_enqueue(&mut storage, 2, &job4).unwrap();

        let jobs = all_unprocessed_drand_jobs(&storage, Order::Ascending, 0, 100).unwrap();
        assert_eq!(jobs, &[job4, job2, job3]);
    }
}
