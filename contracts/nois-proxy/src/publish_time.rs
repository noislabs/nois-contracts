use cosmwasm_std::{StdResult, Storage, Timestamp};

use crate::state::{
    TEST_MODE_NEXT_AFTER, TEST_MODE_NEXT_AFTER_INCREMENT_SECONDS, TEST_MODE_NEXT_AFTER_INIT,
};

/// The publishing time must be at least `SAFETY_MARGIN` in the future. This ensures
/// that in case of clock drifts between consumer chain and wall time only rounds are requested
/// that have not been published yet.
const SAFETY_MARGIN: u64 = 5_000000000; // 5 seconds

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum AfterMode {
    Test,
    BlockTime(Timestamp),
}

/// Calculates the `after` time for the beacon request.
///
/// In test mode this does not use the current block time
/// but a list of constants.
pub fn calculate_after(storage: &mut dyn Storage, mode: AfterMode) -> StdResult<Timestamp> {
    match mode {
        AfterMode::Test => {
            let after = TEST_MODE_NEXT_AFTER
                .may_load(storage)?
                .unwrap_or(TEST_MODE_NEXT_AFTER_INIT);
            TEST_MODE_NEXT_AFTER.save(
                storage,
                &after.plus_seconds(TEST_MODE_NEXT_AFTER_INCREMENT_SECONDS),
            )?;
            Ok(after)
        }
        AfterMode::BlockTime(bt) => Ok(bt.plus_nanos(SAFETY_MARGIN)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::mock_dependencies;

    #[test]
    fn safety_margin_is_correct() {
        let actual = Timestamp::from_nanos(SAFETY_MARGIN);
        let expected = Timestamp::from_nanos(0).plus_seconds(5);
        assert_eq!(actual, expected);
    }

    #[test]
    fn calculate_after_works_for_block_time() {
        let mut deps = mock_dependencies();
        let s = deps.as_mut().storage;

        let after = calculate_after(s, AfterMode::BlockTime(Timestamp::from_nanos(0))).unwrap();
        assert_eq!(after, Timestamp::from_nanos(SAFETY_MARGIN));

        let after = calculate_after(
            s,
            AfterMode::BlockTime(Timestamp::from_nanos(4984338431241435)),
        )
        .unwrap();
        assert_eq!(
            after,
            Timestamp::from_nanos(4984338431241435 + SAFETY_MARGIN)
        );

        let after = calculate_after(s, AfterMode::BlockTime(Timestamp::from_nanos(42))).unwrap();
        assert_eq!(after, Timestamp::from_nanos(42 + SAFETY_MARGIN));
    }

    #[test]
    fn calculate_after_works_for_test_mode() {
        const TEST_VAL1: u64 = 1677687596999999999;
        const TEST_VAL2: u64 = 1677687626999999999;
        const TEST_VAL3: u64 = 1677687656999999999;

        let mut deps = mock_dependencies();
        let s = deps.as_mut().storage;

        let after = calculate_after(s, AfterMode::Test).unwrap();
        assert_eq!(after, Timestamp::from_nanos(TEST_VAL1));
        let after = calculate_after(s, AfterMode::Test).unwrap();
        assert_eq!(after, Timestamp::from_nanos(TEST_VAL2));
        let after = calculate_after(s, AfterMode::Test).unwrap();
        assert_eq!(after, Timestamp::from_nanos(TEST_VAL3));

        // Resets for new storage
        let mut deps = mock_dependencies();
        let s = deps.as_mut().storage;

        let after = calculate_after(s, AfterMode::Test).unwrap();
        assert_eq!(after, Timestamp::from_nanos(TEST_VAL1));
        let after = calculate_after(s, AfterMode::Test).unwrap();
        assert_eq!(after, Timestamp::from_nanos(TEST_VAL2));
        let after = calculate_after(s, AfterMode::Test).unwrap();
        assert_eq!(after, Timestamp::from_nanos(TEST_VAL3));

        // Can be mixed with block height mode for no reason
        let mut deps = mock_dependencies();
        let s = deps.as_mut().storage;

        let after = calculate_after(s, AfterMode::BlockTime(Timestamp::from_nanos(0))).unwrap();
        assert_eq!(after, Timestamp::from_nanos(SAFETY_MARGIN));
        let after = calculate_after(s, AfterMode::Test).unwrap();
        assert_eq!(after, Timestamp::from_nanos(TEST_VAL1));
        let after = calculate_after(s, AfterMode::BlockTime(Timestamp::from_nanos(7))).unwrap();
        assert_eq!(after, Timestamp::from_nanos(7 + SAFETY_MARGIN));
        let after = calculate_after(s, AfterMode::Test).unwrap();
        assert_eq!(after, Timestamp::from_nanos(TEST_VAL2));
        let after = calculate_after(
            s,
            AfterMode::BlockTime(Timestamp::from_nanos(4984338431241435)),
        )
        .unwrap();
        assert_eq!(
            after,
            Timestamp::from_nanos(4984338431241435 + SAFETY_MARGIN)
        );
        let after = calculate_after(s, AfterMode::Test).unwrap();
        assert_eq!(after, Timestamp::from_nanos(TEST_VAL3));
    }
}
