//! Stable event attributes
//!
//! The attributes here should only be changed very carefully as it is likely that clients rely on them.

/// Which entry point/message type was executed
pub const ATTR_ACTION: &str = "action";

pub const EVENT_TYPE_CALLBACK: &str = "nois-callback";

/// "true"/"false" if the callback was successful or not
pub const ATTR_CALLBACK_SUCCESS: &str = "success";

/// The error message (used in case "success" is "false")
pub const ATTR_CALLBACK_ERROR_MSG: &str = "error_msg";
