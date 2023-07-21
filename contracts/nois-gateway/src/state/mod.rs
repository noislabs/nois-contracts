mod config;
mod customers;
mod drand_jobs;
mod requests_log;
mod stats;

pub use config::{Config, CONFIG};
pub use customers::{Customer, CUSTOMERS};
pub use drand_jobs::{
    all_unprocessed_drand_jobs, unprocessed_drand_jobs_dequeue, unprocessed_drand_jobs_enqueue,
    unprocessed_drand_jobs_len, Job,
};
pub use requests_log::{requests_log_add, requests_log_asc, requests_log_desc, RequestLogEntry};
pub use stats::{get_processed_drand_jobs, increment_processed_drand_jobs};

/// Top level storage key. Values must not conflict.
/// Each key is only one byte long to ensure we use the smallest possible storage keys.
#[repr(u8)]
pub enum TopKey {
    Config = b'c',
    Customers = b'C',
    Jobs = b'j',
    JobsLastId = b'J',
    ProcessedDrandJobsCount = b'p',
}

impl TopKey {
    const fn as_str(&self) -> &str {
        let array_ref = unsafe { std::mem::transmute::<_, &[u8; 1]>(self) };
        match core::str::from_utf8(array_ref) {
            Ok(a) => a,
            Err(_) => panic!("Non-utf8 enum value found. Use a-z, A-Z and 0-9"),
        }
    }
}
