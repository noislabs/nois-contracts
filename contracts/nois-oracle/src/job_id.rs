use crate::error::ContractError;
use nois::MAX_JOB_ID_LEN;

pub fn validate_job_id(job_id: &str) -> Result<(), ContractError> {
    if job_id.len() > MAX_JOB_ID_LEN {
        Err(ContractError::JobIdTooLong)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_job_id_works() {
        // We have no good reason to disallow empty
        validate_job_id("").unwrap();
        validate_job_id("abc").unwrap();
        validate_job_id("abcabcabcabcabcaabc2bcabcabcabcabcabcabc").unwrap();
        // a sha256 hash
        validate_job_id("cb480eb3697f39db828d9efa021abe681bfcd72e23894019b8ddb1ab94039081")
            .unwrap();

        let err =
            validate_job_id("abcabcabcabcabca_asfsdfsdgsdgbcbcabcabcabc234t34t3t34gabcabcabc41")
                .unwrap_err();
        assert!(matches!(err, ContractError::JobIdTooLong));
    }
}
