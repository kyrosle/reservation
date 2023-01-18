use chrono::{DateTime, Utc};
use sqlx::postgres::types::PgRange;

use crate::{ReservationQuery, Validator};

use super::{get_timespan, validate_range};

impl ReservationQuery {
    pub fn get_timespan(&self) -> PgRange<DateTime<Utc>> {
        get_timespan(self.start.as_ref(), self.end.as_ref())
    }
}

impl Validator for ReservationQuery {
    fn validate(&self) -> Result<(), crate::Error> {
        validate_range(self.start.as_ref(), self.end.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Bound;

    use prost_types::Timestamp;

    use crate::convert_to_utc_time;

    use super::*;

    #[test]
    fn validate_range_should_allow_correct_range() {
        let start = Timestamp {
            seconds: 1,
            nanos: 0,
        };
        let end = Timestamp {
            seconds: 2,
            nanos: 0,
        };
        assert!(validate_range(Some(&start), Some(&end)).is_ok());
    }

    #[test]
    fn validate_range_should_reject_invalid_range() {
        let start = Timestamp {
            seconds: 2,
            nanos: 0,
        };
        let end = Timestamp {
            seconds: 1,
            nanos: 0,
        };
        assert!(validate_range(Some(&start), Some(&end)).is_err());
    }

    #[test]
    fn get_timespan_should_work_for_valid_start_end() {
        let start = Timestamp {
            seconds: 1,
            nanos: 0,
        };
        let end = Timestamp {
            seconds: 2,
            nanos: 0,
        };
        let range = get_timespan(Some(&start), Some(&end));

        assert_eq!(range.start, Bound::Included(convert_to_utc_time(&start)));
        assert_eq!(range.end, Bound::Included(convert_to_utc_time(&end)));
    }
}
