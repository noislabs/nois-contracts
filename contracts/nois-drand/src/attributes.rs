//! Stable attributes
//!
//! The attributes here should only be changed very carefully as it is likely that clients rely on them.

/// The round of the submitted beacon
pub const ATTR_ROUND: &str = "round";

/// The randomness of the submitted beacon in hex encoding
pub const ATTR_RANDOMNESS: &str = "randomness";

/// The address of the bot submitting this beacon
pub const ATTR_BOT: &str = "bot";
