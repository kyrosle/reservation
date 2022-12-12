use abi::Validator;
use async_trait::async_trait;
use sqlx::{PgPool, Row};

use crate::ReservationManager;
use crate::Rsvp;

#[async_trait]
impl Rsvp for ReservationManager {
    async fn reserve(&self, mut rsvp: abi::Reservation) -> Result<abi::Reservation, abi::Error> {
        rsvp.validate()?;

        let status = abi::ReservationStatus::from_i32(rsvp.status)
            .unwrap_or(abi::ReservationStatus::Pending);

        let timespan = rsvp.get_timespan();

        // generate a insert sql for the reservation
        // execute the sql
        let id = sqlx::query(
            "INSERT INTO rsvp.reservations (user_id, resource_id, timespan, note, status) VALUES ($1, $2, $3, $4, $5::rsvp.reservation_status) RETURNING id"
        )
        .bind(rsvp.user_id.clone())
        .bind(rsvp.resource_id.clone())
        .bind(timespan)
        .bind(rsvp.note.clone())
        .bind(status.to_string())
        .fetch_one(&self.pool)
        .await?.get(0);

        rsvp.id = id;

        Ok(rsvp)
    }

    async fn change_status(
        &self,
        id: crate::ReservationId,
    ) -> Result<abi::Reservation, abi::Error> {
        // if current status is pending, change it to confirmed, otherwise do nothing
        if id == 0 {
            return Err(abi::Error::InvalidReservationId(id));
        }
        let rsvp: abi::Reservation = sqlx::query_as("UPDATE rsvp.reservations  SET status = 'confirmed' WHERE id = $1 AND status = 'pending' RETURNING *").bind(id).fetch_one(&self.pool).await?;
        Ok(rsvp)
    }

    async fn update_note(
        &self,
        id: crate::ReservationId,
        note: String,
    ) -> Result<abi::Reservation, abi::Error> {
        id.validate()?;
        let rsvp: abi::Reservation =
            sqlx::query_as("UPDATE rsvp.reservations SET note = $1 WHERE id = $2 RETURNING *")
                .bind(note)
                .bind(id)
                .fetch_one(&self.pool)
                .await?;
        Ok(rsvp)
    }
    async fn get(&self, id: crate::ReservationId) -> Result<abi::Reservation, abi::Error> {
        // get reservation by id
        id.validate()?;
        let rsvp: abi::Reservation =
            sqlx::query_as("SELECT * FROM rsvp.reservations WHERE id = $1")
                .bind(id)
                .fetch_one(&self.pool)
                .await?;
        Ok(rsvp)
    }

    async fn delete(&self, id: crate::ReservationId) -> Result<(), abi::Error> {
        // delete reservation by id
        id.validate()?;
        sqlx::query("DELETE FROM rsvp.reservations WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn query(
        &self,
        query: abi::ReservationQuery,
    ) -> Result<Vec<abi::Reservation>, abi::Error> {
        let user_id = str_to_option(&query.user_id);
        let resource_id = str_to_option(&query.resource_id);
        let range = query.timespan();
        let status = abi::ReservationStatus::from_i32(query.status)
            .unwrap_or(abi::ReservationStatus::Pending);
        let rsvps = sqlx::query_as(
            "SELECT * FROM rsvp.query($1,$2,$3,$4::rsvp.reservation_status,$5,$6,$7)",
        )
        .bind(user_id)
        .bind(resource_id)
        .bind(range)
        .bind(status.to_string())
        .bind(query.page)
        .bind(query.desc)
        .bind(query.page_size)
        .fetch_all(&self.pool)
        .await?;
        Ok(rsvps)
    }
    async fn filter(
        &self,
        filter: abi::ReservationFilter,
    ) -> Result<(abi::FilterPager, Vec<abi::Reservation>), abi::Error> {
        let user_id = str_to_option(&filter.user_id);
        let resource_id = str_to_option(&filter.resource_id);
        let status = abi::ReservationStatus::from_i32(filter.status)
            .unwrap_or(abi::ReservationStatus::Pending);

        let rsvps: Vec<abi::Reservation> =
            sqlx::query_as("SELECT * FROM rsvp.filter($1,$2,$3::rsvp.reservation_status,$4,$5,$6)")
                .bind(user_id)
                .bind(resource_id)
                .bind(status.to_string())
                .bind(filter.cursor)
                .bind(filter.desc)
                .bind(filter.page_size)
                .fetch_all(&self.pool)
                .await?;

        let page_size = if filter.page_size < 10 || filter.page_size > 100 {
            10
        } else {
            filter.page_size
        };

        // if the first id is current cursor, then we have prev, we start from 1
        // if end - first > page_size, then we have next we end at len - 1
        let has_prev = !rsvps.is_empty() && rsvps[0].id == filter.cursor;
        let start = if has_prev { 1 } else { 0 };

        let has_next = (rsvps.len() - start) as i32 > page_size;
        let end = if has_next {
            rsvps.len() - 1
        } else {
            rsvps.len()
        };

        let prev = if has_prev { rsvps[start - 1].id } else { -1 };
        let next = if has_next { rsvps[end - 1].id } else { -1 };

        // TODO: optimize this clone
        let result = rsvps[start..end].to_vec();

        let pager = abi::FilterPager {
            next,
            prev,
            // TODO: how to get total efficiently?
            total: 0,
        };

        Ok((pager, result))
    }
}

impl ReservationManager {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn str_to_option(s: &str) -> Option<&str> {
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

#[cfg(test)]
mod tests {
    use abi::{
        Reservation, ReservationConflict, ReservationConflictInfo, ReservationFilterBuilder,
        ReservationQueryBuilder, ReservationWindow,
    };

    use super::*;

    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn reserve_should_work_for_valid_window() {
        let (rsvp, _manager) = make_kyros_reservation(migrated_pool.clone()).await;
        assert!(rsvp.id != 0);
    }
    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn reserve_conflict_should_reject() {
        let (_, manager) = make_kyros_reservation(migrated_pool.clone()).await;

        let rsvp2 = abi::Reservation::new(
            "alice",
            "ocean-view-room-417",
            "2022-12-26T15:00:00-0700".parse().unwrap(),
            "2022-12-30T12:00:00-0700".parse().unwrap(),
            "Hello.",
        );
        let err = manager.reserve(rsvp2).await.unwrap_err();

        let info = ReservationConflictInfo::Parsed(ReservationConflict {
            new: ReservationWindow {
                rid: "ocean-view-room-417".to_string(),
                start: "2022-12-26T15:00:00-0700".parse().unwrap(),
                end: "2022-12-30T12:00:00-0700".parse().unwrap(),
            },
            old: ReservationWindow {
                rid: "ocean-view-room-417".to_string(),
                start: "2022-12-25T15:00:00-0700".parse().unwrap(),
                end: "2022-12-28T12:00:00-0700".parse().unwrap(),
            },
        });

        assert_eq!(err, abi::Error::ConflictReservation(info));
    }

    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn reserve_change_status_should_work() {
        let (rsvp, manager) = make_alice_reservation(migrated_pool.clone()).await;

        let rsvp = manager.change_status(rsvp.id).await.unwrap();
        assert_eq!(rsvp.status, abi::ReservationStatus::Confirmed as i32);
    }
    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn reserve_change_status_not_pending_should_do_nothing() {
        let (rsvp, manager) = make_alice_reservation(migrated_pool.clone()).await;

        let rsvp = manager.change_status(rsvp.id).await.unwrap();

        // change status again should do nothing
        let err = manager.change_status(rsvp.id).await.unwrap_err();

        assert_eq!(err, abi::Error::NotFound);
    }
    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn update_note_should_work() {
        let (rsvp, manager) = make_alice_reservation(migrated_pool.clone()).await;

        let rsvp = manager
            .update_note(rsvp.id, "hello world".into())
            .await
            .unwrap();

        assert_eq!(rsvp.note, "hello world");
    }
    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn get_reservation_should_work() {
        let (rsvp, manager) = make_alice_reservation(migrated_pool.clone()).await;

        let rsvp1 = manager.get(rsvp.id).await.unwrap();

        assert_eq!(rsvp, rsvp1);
    }
    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn delete_reservation_should_work() {
        let (rsvp, manager) = make_alice_reservation(migrated_pool.clone()).await;

        manager.delete(rsvp.id).await.unwrap();

        let rsvp1 = manager.get(rsvp.id).await.unwrap_err();

        assert_eq!(rsvp1, abi::Error::NotFound);
    }
    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn query_reservation_should_work() {
        let (rsvp, manager) = make_alice_reservation(migrated_pool.clone()).await;

        // ---

        let mut query = ReservationQueryBuilder::default()
            .user_id("alice")
            .resource_id("")
            .status(abi::ReservationStatus::Pending as i32)
            .start(
                "2021-11-01T15:00:00-0700"
                    .parse::<::prost_types::Timestamp>()
                    .unwrap(),
            )
            .end(
                "2023-12-31T12:00:00-0700"
                    .parse::<::prost_types::Timestamp>()
                    .unwrap(),
            )
            .clone();

        let rsvps = manager.query(query.clone().build().unwrap()).await.unwrap();
        assert_eq!(rsvps.len(), 1);
        assert_eq!(rsvps[0], rsvp);

        // ---

        // if the window is not in range, should return empty.
        let query = query.start(
            "2023-11-01T15:00:00-0700"
                .parse::<::prost_types::Timestamp>()
                .unwrap(),
        );

        let rsvps = manager.query(query.clone().build().unwrap()).await.unwrap();
        assert_eq!(rsvps.len(), 0);

        // restore the query
        let query = query.start(
            "2021-11-01T15:00:00-0700"
                .parse::<::prost_types::Timestamp>()
                .unwrap(),
        );

        // ---

        // if the status is not correct, should return empty.
        let query = query.status(abi::ReservationStatus::Confirmed as i32);

        let rsvps = manager.query(query.clone().build().unwrap()).await.unwrap();
        assert_eq!(rsvps.len(), 0);

        // ---

        // set the status to be confirmed, then test the Confirmed status, of this reservation
        let _ = manager.change_status(rsvp.id).await.unwrap();

        let rsvps = manager.query(query.clone().build().unwrap()).await.unwrap();
        assert_eq!(rsvps.len(), 1);

        // ---
    }

    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn filter_reservation_should_work() {
        let (rsvp, manager) = make_alice_reservation(migrated_pool.clone()).await;
        let filter = ReservationFilterBuilder::default()
            .user_id("alice")
            .status(abi::ReservationStatus::Pending as i32)
            .clone();

        let (pager, rsvps) = manager
            .filter(filter.clone().build().unwrap())
            .await
            .unwrap();
        assert_eq!(pager.prev, -1);
        assert_eq!(pager.next, -1);
        assert_eq!(rsvps.len(), 1);
        assert_eq!(rsvps[0], rsvp);
    }

    // private none test functions
    async fn make_alice_reservation(pool: PgPool) -> (Reservation, ReservationManager) {
        make_reservation(
            pool,
            "alice",
            "ixia-test-1",
            "2023-01-25T15:00:00-0700",
            "2023-02-25T12:00:00-0700",
            "I need to book this for xyz project for a month.",
        )
        .await
    }
    async fn make_kyros_reservation(pool: PgPool) -> (Reservation, ReservationManager) {
        make_reservation(
            pool,
            "kyros",
            "ocean-view-room-417",
            "2022-12-25T15:00:00-0700",
            "2022-12-28T12:00:00-0700",
            "I'll arrive at 3pm. Please help to upgrade to executive room if possible.",
        )
        .await
    }
    async fn make_reservation(
        pool: PgPool,
        uid: &str,
        rid: &str,
        start: &str,
        end: &str,
        note: &str,
    ) -> (Reservation, ReservationManager) {
        let manager = ReservationManager::new(pool.clone());
        let rsvp =
            abi::Reservation::new(uid, rid, start.parse().unwrap(), end.parse().unwrap(), note);

        (manager.reserve(rsvp).await.unwrap(), manager)
    }
}
