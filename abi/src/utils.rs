use chrono::{DateTime, NaiveDateTime, Utc};
use prost_types::Timestamp;

pub fn convert_to_utc_time(ts: &Timestamp) -> DateTime<Utc> {
    DateTime::<Utc>::from_utc(
        NaiveDateTime::from_timestamp_opt(ts.seconds, ts.nanos as _).unwrap(),
        Utc,
    )
}

pub fn convert_to_timestamp(dt: &DateTime<Utc>) -> Timestamp {
    Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subset_nanos() as _,
    }
}
