mod manager;

use abi::ReservationId;
use async_trait::async_trait;
use sqlx::PgPool;
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct ReservationManager {
    pool: PgPool,
}

#[async_trait]
pub trait Rsvp {
    /// make a reservation
    async fn reserve(&self, rsvp: abi::Reservation) -> Result<abi::Reservation, abi::Error>;
    /// change reservation status (if current status is pending, change it to confirmed)
    async fn change_status(&self, rsvp: ReservationId) -> Result<abi::Reservation, abi::Error>;
    /// update note
    async fn update_note(
        &self,
        id: ReservationId,
        note: String,
    ) -> Result<abi::Reservation, abi::Error>;
    /// delete reservation
    async fn delete(&self, id: ReservationId) -> Result<abi::Reservation, abi::Error>;
    /// get reservation by id
    async fn get(&self, id: ReservationId) -> Result<abi::Reservation, abi::Error>;
    /// query reservations
    async fn query(
        &self,
        query: abi::ReservationQuery,
    ) -> mpsc::Receiver<Result<abi::Reservation, abi::Error>>;
    /// query reservation order by reservation id
    async fn filter(
        &self,
        filter: abi::ReservationFilter,
    ) -> Result<(abi::FilterPager, Vec<abi::Reservation>), abi::Error>;
}
