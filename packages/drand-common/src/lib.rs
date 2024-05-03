mod network;
mod rounds;
pub mod testing;

pub use network::DrandNetwork;
pub use rounds::{is_incentivized, round_after, time_of_round};
