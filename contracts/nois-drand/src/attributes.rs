//! Stable event attributes
//!
//! The attributes here should only be changed very carefully as it is likely that clients rely on them.

/// The round of the submitted beacon
pub const ATTR_ROUND: &str = "round";

/// The randomness of the submitted beacon in hex encoding
pub const ATTR_RANDOMNESS: &str = "randomness";

/// The address of the bot submitting this beacon
pub const ATTR_BOT: &str = "bot";

/// The number of reward points earned for this submission.
/// Points are counted regardless of the actual payout. E.g.
/// a bot that is not allow-listed can see the points but gets a zero
/// payount amount.
pub const ATTR_REWARD_POINTS: &str = "reward_points";

/// The amount of tokens actually paid out as rewards.
/// This can differ from the number calculated by points, e.g.
/// when the contract has an insufficient balance.
pub const ATTR_REWARD_PAYOUT: &str = "reward_payout";
