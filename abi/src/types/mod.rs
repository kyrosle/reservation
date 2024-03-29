use std::ops::Bound;

use chrono::{DateTime, Utc};
use prost_types::Timestamp;
use sqlx::postgres::types::PgRange;

use crate::{convert_to_utc_time, Error};

mod request;
mod reservation;
mod reservation_filter;
mod reservation_query;
mod reservation_status;

pub fn validate_range(start: Option<&Timestamp>, end: Option<&Timestamp>) -> Result<(), Error> {
    if start.is_none() || end.is_none() {
        return Err(Error::InvalidTime);
    }

    // let start = convert_to_utc_time(self.start.as_ref().unwrap().clone());
    // let end = convert_to_utc_time(self.end.as_ref().unwrap().clone());

    if start.unwrap().seconds >= end.unwrap().seconds {
        return Err(Error::InvalidTime);
    }
    Ok(())
}
pub fn get_timespan(start: Option<&Timestamp>, end: Option<&Timestamp>) -> PgRange<DateTime<Utc>> {
    let start = convert_to_utc_time(start.unwrap());
    let end = convert_to_utc_time(end.unwrap());
    PgRange {
        start: Bound::Included(start),
        end: Bound::Included(end),
    }
}
