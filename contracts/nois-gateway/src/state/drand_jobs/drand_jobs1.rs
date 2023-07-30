use cosmwasm_schema::cw_serde;
#[cfg(test)]
use cosmwasm_std::{from_slice, Order};
use cosmwasm_std::{Binary, StdResult, Storage};
use cw_storage_plus::Deque;

const UNPROCESSED_DRAND_JOBS_KEY_LEN: u16 = 24;

pub const DRAND_JOBS_V1_START: &[u8] = b"\x00\x18drand_jobs_up_0000000000";
pub const DRAND_JOBS_V1_END: &[u8] = b"\x00\x18drand_jobs_up_9999999999";

/// This is the length of the storage key of a meta field
/// of the Deque storage type. The 2 is the length-prefixed encoding.
/// The 1 is the "h" or "t".
#[cfg(test)]
const DEQUE_META_FIELD_LEN: usize = 2 + (UNPROCESSED_DRAND_JOBS_KEY_LEN as usize) + 1;

#[cw_serde]
pub struct Job {
    /// A RNG specific randomness source identifier, e.g. `drand:<network id>:<round>`
    pub source_id: String,
    // The channel the job came from and we have to send the response to
    pub channel: String,
    pub origin: Binary,
}

#[inline]
fn unprocessed_drand_jobs_key(round: u64) -> String {
    // "up" for unprocessed
    // At a frequency of 1 beacon/second, 10 decimal places is sufficient for 300 years.
    let key = format!("drand_jobs_up_{:0>10}", round);
    debug_assert_eq!(key.len(), UNPROCESSED_DRAND_JOBS_KEY_LEN as usize);
    key
}

/// Add an element to the unprocessed drand jobs queue of this round
#[cfg(test)]
pub fn unprocessed_drand_jobs_enqueue(
    storage: &mut dyn Storage,
    round: u64,
    value: &Job,
) -> StdResult<()> {
    let prefix = unprocessed_drand_jobs_key(round);
    Deque::new(&prefix).push_back(storage, value)
}

/// Remove an element from the unprocessed drand jobs queue of this round
pub fn unprocessed_drand_jobs_dequeue(
    storage: &mut dyn Storage,
    round: u64,
) -> StdResult<Option<Job>> {
    let prefix = unprocessed_drand_jobs_key(round);
    Deque::new(&prefix).pop_front(storage)
}

/// Gets the number of unprocessed drand jobs queue of this round
pub fn unprocessed_drand_jobs_len(storage: &dyn Storage, round: u64) -> StdResult<u32> {
    let prefix = unprocessed_drand_jobs_key(round);
    Deque::<Job>::new(&prefix).len(storage)
}

#[cfg(test)]
pub fn all_unprocessed_drand_jobs(
    storage: &dyn Storage,
    order: Order,
    offset: usize,
    limit: usize,
) -> StdResult<Vec<Job>> {
    storage
        .range(Some(DRAND_JOBS_V1_START), Some(DRAND_JOBS_V1_END), order)
        .filter(|(key, _value)| key.len() != DEQUE_META_FIELD_LEN)
        .skip(offset)
        .take(limit)
        .map(|(_key, value)| from_slice::<Job>(&value))
        .collect::<StdResult<Vec<_>>>()
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::MockStorage;

    use super::*;

    #[test]
    fn unprocessed_drand_jobs_key_works() {
        assert_eq!(unprocessed_drand_jobs_key(0), "drand_jobs_up_0000000000");
        assert_eq!(unprocessed_drand_jobs_key(1), "drand_jobs_up_0000000001");
        assert_eq!(unprocessed_drand_jobs_key(42), "drand_jobs_up_0000000042");
        assert_eq!(
            unprocessed_drand_jobs_key(2879178),
            "drand_jobs_up_0002879178"
        );
        assert_eq!(
            unprocessed_drand_jobs_key(9876543210),
            "drand_jobs_up_9876543210"
        );
    }

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
