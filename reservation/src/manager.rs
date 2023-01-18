use abi::convert_to_utc_time;
use abi::DbConfig;
use abi::Normalizer;
use abi::ToSql;
use abi::Validator;
use async_trait::async_trait;
use futures::StreamExt;
use sqlx::postgres::PgPoolOptions;
use sqlx::Either;
use sqlx::{PgPool, Row};
use tokio::sync::mpsc;
use tracing::info;
use tracing::log::warn;

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

    async fn delete(&self, id: crate::ReservationId) -> Result<abi::Reservation, abi::Error> {
        // delete reservation by id
        id.validate()?;
        let rsvp: abi::Reservation =
            sqlx::query_as("DELETE FROM rsvp.reservations WHERE id = $1 RETURNING *")
                .bind(id)
                .fetch_one(&self.pool)
                .await?;
        Ok(rsvp)
    }

    async fn query(
        &self,
        query: abi::ReservationQuery,
    ) -> mpsc::Receiver<Result<abi::Reservation, abi::Error>> {
        let user_id = string_to_option(&query.user_id);
        let resource_id = string_to_option(&query.resource_id);
        // let range = query.get_timespan();
        let start = query.start.map(|v| convert_to_utc_time(&v));
        let end = query.end.map(|v| convert_to_utc_time(&v));
        let status = abi::ReservationStatus::from_i32(query.status)
            .unwrap_or(abi::ReservationStatus::Pending);

        let pool = self.pool.clone();
        let (tx, rx) = mpsc::channel(128);

        tokio::spawn(async move {
            let mut rsvps = sqlx::query_as(
                "SELECT * FROM rsvp.query($1,$2,$3,$4,$5::rsvp.reservation_status,$6)",
            )
            .bind(user_id)
            .bind(resource_id)
            .bind(start)
            .bind(end)
            .bind(status.to_string())
            .bind(query.desc)
            .fetch_many(&pool);
            while let Some(ret) = rsvps.next().await {
                match ret {
                    Ok(Either::Left(r)) => {
                        info!("Query result: {:?}", r);
                    }
                    Ok(Either::Right(r)) => {
                        // rx is dropped, so client disconnected
                        if tx.send(Ok(r)).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        warn!("Query error: {e:?}");
                        // rx is dropped, so client disconnected
                        if tx.send(Err(e.into())).await.is_err() {
                            break;
                        }
                        break;
                    }
                }
            }
        });
        rx
    }

    async fn filter(
        &self,
        mut filter: abi::ReservationFilter,
    ) -> Result<(abi::FilterPager, Vec<abi::Reservation>), abi::Error> {
        filter.normalize()?;

        let sql = filter.to_sql();
        let rsvps: Vec<abi::Reservation> = sqlx::query_as(&sql).fetch_all(&self.pool).await?;

        let mut rsvps = rsvps.into_iter().collect();
        let pager = filter.get_pager(&mut rsvps)?;

        Ok((pager, rsvps.into_iter().collect()))
    }
}

impl ReservationManager {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    pub async fn from_config(config: &DbConfig) -> Result<Self, abi::Error> {
        let url = config.url();
        let pool = PgPoolOptions::default()
            .max_connections(config.max_connections)
            .connect(&url)
            .await?;
        Ok(Self::new(pool))
    }
}

fn str_to_option(s: &str) -> Option<&str> {
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

fn string_to_option(s: &str) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s.into())
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

        let rsvp2 = abi::Reservation::new_pending(
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
        let (mut rsvp, manager) = make_alice_reservation(migrated_pool.clone()).await;

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

        let mut rx = manager.query(query.clone().build().unwrap()).await;
        assert_eq!(rx.recv().await, Some(Ok(rsvp.clone())));
        assert_eq!(rx.recv().await, None);

        // ---

        // if the window is not in range, should return empty.
        let query = query.start(
            "2023-11-01T15:00:00-0700"
                .parse::<::prost_types::Timestamp>()
                .unwrap(),
        );

        let mut rx = manager.query(query.clone().build().unwrap()).await;
        assert_eq!(rx.recv().await, None);

        // restore the query
        let query = query.start(
            "2021-11-01T15:00:00-0700"
                .parse::<::prost_types::Timestamp>()
                .unwrap(),
        );

        // ---

        // if the status is not correct, should return empty.
        let query = query.status(abi::ReservationStatus::Confirmed as i32);

        let mut rx = manager.query(query.clone().build().unwrap()).await;
        assert_eq!(rx.recv().await, None);

        // ---

        // set the status to be confirmed, then test the Confirmed status, of this reservation
        let _ = manager.change_status(rsvp.id).await.unwrap();
        rsvp.set_status(abi::ReservationStatus::Confirmed);

        let mut rx = manager.query(query.clone().build().unwrap()).await;
        assert_eq!(rx.recv().await, Some(Ok(rsvp)));
        assert_eq!(rx.recv().await, None);

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
        assert_eq!(pager.prev, None);
        assert_eq!(pager.next, None);
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
        let rsvp = abi::Reservation::new_pending(
            uid,
            rid,
            start.parse().unwrap(),
            end.parse().unwrap(),
            note,
        );

        (manager.reserve(rsvp).await.unwrap(), manager)
    }
}
