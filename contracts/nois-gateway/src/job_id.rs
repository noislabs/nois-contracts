use crate::error::ContractError;

/// This is a JSON encoded RequestBeaconOrigin with a contract address and
/// a job ID (64 bytes). So something like 200 bytes should be sufficient here.
/// Add some buffer for future entensions but in general the proxy should keep
/// this small
const MAX_ORIGIN_LEN: usize = 512;

pub fn validate_origin(origin: &[u8]) -> Result<(), ContractError> {
    if origin.len() > MAX_ORIGIN_LEN {
        Err(ContractError::OriginTooLong)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_origin_works() {
        // We have no good reason to disallow empty
        validate_origin(b"").unwrap();
        validate_origin(b"abc").unwrap();
        validate_origin(b"abcabcabcabcabcaabc2bcabcabcabcabcabcabc").unwrap();
        // max
        validate_origin(b"abcabcabcabcabca_asfsdfsdgsdgbcbcabcabcabc234t34t3t34gabcabcabc4abcabcabcabcabca_asfsdfsdgsdgbcbcabcabcabc234t34t3t34gabcabcabc4abcabcabcabcabca_asfsdfsdgsdgbcbcabcabcabc234t34t3t34gabcabcabc4abcabcabcabcabca_asfsdfsdgsdgbcbcabcabcabc234t34t3t34gabcabcabc4abcabcabcabcabca_asfsdfsdgsdgbcbcabcabcabc234t34t3t34gabcabcabc4abcabcabcabcabca_asfsdfsdgsdgbcbcabcabcabc234t34t3t34gabcabcabc4abcabcabcabcabca_asfsdfsdgsdgbcbcabcabcabc234t34t3t34gabcabcabc4abcabcabcabcabca_asfsdfsdgsdgbcbcabcabcabc234t34t3t34gabcabcabc4").unwrap();

        let err =
        validate_origin(b"abcabcabcabcabca_asfsdfsdgsdgbcbcabcabcabc234t34t3t34gabcabcabc4abcabcabcabcabca_asfsdfsdgsdgbcbcabcabcabc234t34t3t34gabcabcabc4abcabcabcabcabca_asfsdfsdgsdgbcbcabcabcabc234t34t3t34gabcabcabc4abcabcabcabcabca_asfsdfsdgsdgbcbcabcabcabc234t34t3t34gabcabcabc4abcabcabcabcabca_asfsdfsdgsdgbcbcabcabcabc234t34t3t34gabcabcabc4abcabcabcabcabca_asfsdfsdgsdgbcbcabcabcabc234t34t3t34gabcabcabc4abcabcabcabcabca_asfsdfsdgsdgbcbcabcabcabc234t34t3t34gabcabcabc4abcabcabcabcabca_asfsdfsdgsdgbcbcabcabcabc234t34t3t34gabcabcabc41")
                .unwrap_err();
        assert!(matches!(err, ContractError::OriginTooLong));
    }
}
