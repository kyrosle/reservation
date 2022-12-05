use sqlx::{postgres::types::PgRange, types::uuid::Timestamp};

use crate::{ReservationManager, Rsvp, error::ReservationError};

impl Rsvp for ReservationManager {
    async fn reserve(&self, rsvp: abi::Reservation) -> Result<(), crate::error::ReservationError> {
        if rsvp.start.is_none() || rsvp.end.is_none() {
            return Err(ReservationError::InvalidTime);
        }

        let start = rsvp.start.unwrap();
        let end = rsvp.end.unwrap();

        if start <= end {
            return Err(ReservationError::InvalidTime);
        }

        let range: PgRange<Timestamp> = (start..end).into();
        let timespan = PgRange::new(rsvp.)
        // generate a insert sql for the reservation
        let sql = "INSERT INTO reservation (user_id, resource_id, timespan, note, status) VALUES ($1, $2, $3, $4, $5) RETURNING id";
        let id = sqlx::query!(
            sql,
            rsvp.user_id,
            rsvp.resource_id,
            rsvp.timespan,
            rsvp.note,
            rsvp.status,
        )
        .fetch_one(&self.pool)
        .await?
        .id;
        Ok(())
    }

    async fn change_status(
        &self,
        rsvp: crate::ReservationId,
    ) -> Result<abi::Reservation, crate::error::ReservationError> {
        todo!()
    }

    async fn update_note(
        &self,
        id: crate::ReservationId,
        note: String,
    ) -> Result<abi::Reservation, crate::error::ReservationError> {
        todo!()
    }

    async fn delete(
        &self,
        id: crate::ReservationId,
    ) -> Result<abi::Reservation, crate::error::ReservationError> {
        todo!()
    }

    async fn query(
        &self,
        query: abi::ReservationQuery,
    ) -> Result<Vec<abi::Reservation>, crate::error::ReservationError> {
        todo!()
    }

    async fn get(
        &self,
        id: crate::ReservationId,
    ) -> Result<abi::Reservation, crate::error::ReservationError> {
        todo!()
    }
}
