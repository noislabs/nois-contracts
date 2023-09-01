mod drand_jobs2;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::Binary;

#[cw_serde]
pub struct Job {
    /// A RNG specific randomness source identifier, e.g. `drand:<network id>:<round>`
    pub source_id: String,
    // The channel the job came from and we have to send the response to
    pub channel: String,
    pub origin: Binary,
}

// New jobs always go to v2
pub use drand_jobs2::{
    all_unprocessed_drand_jobs, unprocessed_drand_jobs_dequeue, unprocessed_drand_jobs_enqueue,
    unprocessed_drand_jobs_len,
};

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

        unprocessed_drand_jobs_enqueue(&mut storage, round, &job1).unwrap();
        unprocessed_drand_jobs_enqueue(&mut storage, round, &job2).unwrap();
        unprocessed_drand_jobs_enqueue(&mut storage, round, &job3).unwrap();
        unprocessed_drand_jobs_enqueue(&mut storage, round, &job4).unwrap();

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
        unprocessed_drand_jobs_enqueue(&mut storage, round, &job1).unwrap();
        assert_eq!(unprocessed_drand_jobs_len(&storage, round).unwrap(), 1);
        unprocessed_drand_jobs_enqueue(&mut storage, round, &job2).unwrap();
        assert_eq!(unprocessed_drand_jobs_len(&storage, round).unwrap(), 2);
        unprocessed_drand_jobs_enqueue(&mut storage, round, &job3).unwrap();
        assert_eq!(unprocessed_drand_jobs_len(&storage, round).unwrap(), 3);
        unprocessed_drand_jobs_enqueue(&mut storage, round, &job4).unwrap();
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
