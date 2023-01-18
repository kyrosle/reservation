use std::collections::VecDeque;

use crate::{
    Error, FilterPager, Normalizer, Reservation, ReservationFilter, ReservationFilterBuilder,
    ReservationStatus, ToSql, Validator,
};

impl ReservationFilterBuilder {
    pub fn build(&self) -> Result<ReservationFilter, Error> {
        let mut filter = self
            .private_build()
            .expect("failed to build ReservationFilter");
        filter.normalize()?;
        Ok(filter)
    }
}

impl Validator for ReservationFilter {
    fn validate(&self) -> Result<(), crate::Error> {
        if self.page_size < 10 || self.page_size > 100 {
            return Err(Error::InvalidPageSize(self.page_size));
        }

        if let Some(cursor) = self.cursor {
            if cursor < 0 {
                return Err(Error::InvalidCursor(cursor));
            }
        }

        ReservationStatus::from_i32(self.status).ok_or(Error::InvalidStatus(self.status))?;

        Ok(())
    }
}

impl Normalizer for ReservationFilter {
    fn do_normalize(&mut self) {
        if self.status == ReservationStatus::Unknown as i32 {
            self.status = ReservationStatus::Pending as i32;
        }
    }
}

/*
-- we can filter one more items one for staring, one for ending.
-- If starting exsiting, then we have perious page,
-- If ending existing, then we have next page.
CREATE OR REPLACE FUNCTION rsvp.filter(
    uid text,
    rid text,
    status rsvp.reservation_status,
    cursor bigint DEFAULT NULL,
    is_desc bool DEFAULT FALSE,
    page_size bigint DEFAULT 10
) RETURNS TABLE (LIKE rsvp.reservations) AS $$
DECLARE
    _sql text;
    _offset bigint;
BEGIN
    -- if page_size is not between 10 and 100, set it to 10
    IF page_size < 10 OR page_size > 100 THEN
        page_size := 10;
    END IF;

    -- if cursor is NULL, set it to 0 if is_desc is false, or to 2^63 - 1 if is_desc is true
    IF cursor IS NULL OR cursor < 0  THEN
        IF is_desc THEN
            cursor := 9223372036854775807;
        ELSE
            cursor := 0;
        END IF;
    END IF;

    -- format the query based on parameters
    _sql := format('SELECT * FROM rsvp.reservations WHERE %s AND status = %L AND %s ORDER BY id %s LIMIT %L::integer',
        CASE
            WHEN is_desc THEN 'id <= ' || cursor
            ELSE 'id >= ' || cursor
        END,
        status,
        CASE
            WHEN uid IS NULL AND rid IS NULL THEN 'TRUE'
            WHEN uid IS NULL THEN 'resource_id = ' || quote_literal(rid)
            WHEN rid IS NULL THEN 'user_id = ' || quote_literal(uid)
            ELSE 'user_id = ' || quote_literal(uid) || ' AND resource_id = ' || quote_literal(rid)
        END,
        CASE
            WHEN is_desc THEN 'DESC'
            ELSE 'ASC'
        END,
        page_size + 1
    );
    -- log the sql
    RAISE NOTICE '%', _sql;

    -- excute the query
    RETURN QUERY EXECUTE _sql;
END;
$$ LANGUAGE plpgsql;
*/
impl ReservationFilter {
    pub fn get_pager(&self, rsvps: &mut VecDeque<Reservation>) -> Result<FilterPager, Error> {
        let has_prev = self.cursor.is_some();
        let start = if has_prev { rsvps.pop_front() } else { None };

        let has_next = rsvps.len() as i64 > self.page_size;
        let end = if has_next { rsvps.pop_back() } else { None };

        let inner_id = |r: Reservation| r.id;
        let pager = FilterPager {
            prev: start.map(inner_id),
            next: end.map(inner_id),
            total: None,
        };

        Ok(pager)
    }
    pub fn get_cursor(&self) -> i64 {
        self.cursor.unwrap_or(if self.desc { i64::MAX } else { 0 })
    }
    pub fn get_status(&self) -> ReservationStatus {
        ReservationStatus::from_i32(self.status).unwrap()
    }
}
impl ToSql for ReservationFilter {
    fn to_sql(&self) -> String {
        let middle_plus = if self.cursor.is_none() { 0 } else { 1 };
        let limit = self.page_size + 1 + middle_plus;

        let status = self.get_status();

        let cursor_cond = if self.desc {
            format!("id <= {}", self.get_cursor())
        } else {
            format!("id >= {}", self.get_cursor())
        };

        let user_resource_cond = match (self.user_id.is_empty(), self.resource_id.is_empty()) {
            (true, true) => "TRUE".into(),
            (true, false) => format!("resource_id = '{}'", self.resource_id),
            (false, true) => format!("user_id = '{}'", self.user_id),
            (false, false) => format!(
                "user_id = '{}' AND resource_id = '{}'",
                self.user_id, self.resource_id
            ),
        };

        let direction = if self.desc { "DESC" } else { "ASC" };

        format!("SELECT * FROM rsvp.reservations WHERE status = '{status}'::rsvp.reservation_status AND {cursor_cond} AND {user_resource_cond} ORDER BY id {direction} LIMIT {limit}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ReservationFilterBuilder;

    #[test]
    fn filter_should_generate_correct_sql() {
        let filter = ReservationFilterBuilder::default()
            .user_id("tyr")
            .build()
            .unwrap();

        let sql = filter.to_sql();
        assert_eq!(
            sql,
            "SELECT * FROM rsvp.reservations WHERE status = 'pending'::rsvp.reservation_status AND id >= 0 AND user_id = 'tyr' ORDER BY id ASC LIMIT 11"
        );

        let filter = ReservationFilterBuilder::default()
            .user_id("tyr")
            .resource_id("test")
            .build()
            .unwrap();
        let sql = filter.to_sql();
        assert_eq!(
            sql,
            "SELECT * FROM rsvp.reservations WHERE status = 'pending'::rsvp.reservation_status AND id >= 0 AND user_id = 'tyr' AND resource_id = 'test' ORDER BY id ASC LIMIT 11"
        );

        let filter = ReservationFilterBuilder::default()
            .desc(true)
            .build()
            .unwrap();

        let sql = filter.to_sql();
        assert_eq!(
            sql,
            "SELECT * FROM rsvp.reservations WHERE status = 'pending'::rsvp.reservation_status AND id <= 9223372036854775807 AND TRUE ORDER BY id DESC LIMIT 11"
        );

        let filter = ReservationFilterBuilder::default()
            .user_id("tyr")
            .cursor(100)
            .build()
            .unwrap();

        let sql = filter.to_sql();
        assert_eq!(
            sql,
            "SELECT * FROM rsvp.reservations WHERE status = 'pending'::rsvp.reservation_status AND id >= 100 AND user_id = 'tyr' ORDER BY id ASC LIMIT 12"
        );

        let filter = ReservationFilterBuilder::default()
            .user_id("tyr")
            .cursor(10)
            .desc(true)
            .build()
            .unwrap();

        let sql = filter.to_sql();
        assert_eq!(
            sql,
            "SELECT * FROM rsvp.reservations WHERE status = 'pending'::rsvp.reservation_status AND id <= 10 AND user_id = 'tyr' ORDER BY id DESC LIMIT 12"
        );
    }
}
