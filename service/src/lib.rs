// mod config;

// use abi::{
//     reservation_service_server::ReservationService, CancelRequest, CancelResponse, ConfirmRequest,
//     ConfirmResponse, FilterRequest, FilterResponse, GetRequest, GetResponse, ListenRequest,
//     QueryRequest, Reservation, ReserveRequest, ReserveResponse, UpdateRequest, UpdateResponse,
// };
// use config::Config;
// use futures::Stream;
// use reservation::{ReservationManager, Rsvp};
// use std::{ops::Deref, pin::Pin};
// use tonic::{async_trait, Request, Response, Status};
// pub struct RsvpService(ReservationManager);

// impl Deref for RsvpService {
//     type Target = ReservationManager;

//     fn deref(&self) -> &Self::Target {
//         &self.0
//     }
// }

// impl RsvpService {
//     pub fn new(config: Config) -> Self {
//         // Self(ReservationManager::new())
//         todo!()
//     }
// }

// type ReservationStream = Pin<Box<dyn Stream<Item = Result<Reservation, Status>> + Send>>;

// #[async_trait]
// impl ReservationService for RsvpService {
//     /// make a reservation
//     async fn reserve(
//         &self,
//         request: Request<ReserveRequest>,
//     ) -> Result<Response<ReserveResponse>, Status> {
//         todo!()
//     }
//     /// confirm a pending reservation, if reservation is not pending, do nothing
//     async fn confirm(
//         &self,
//         request: Request<ConfirmRequest>,
//     ) -> Result<Response<ConfirmResponse>, Status> {
//         todo!()
//     }
//     /// update the reservation note
//     async fn update(
//         &self,
//         request: Request<UpdateRequest>,
//     ) -> Result<Response<UpdateResponse>, Status> {
//         todo!()
//     }
//     /// cancel a reservation
//     async fn cancel(
//         &self,
//         request: Request<CancelRequest>,
//     ) -> Result<Response<CancelResponse>, Status> {
//         todo!()
//     }
//     /// get a reservation by id
//     async fn get(&self, request: Request<GetRequest>) -> Result<Response<GetResponse>, Status> {
//         todo!()
//     }
//     /// Server streaming response type for the query method.
//     type queryStream = ReservationStream;
//     /// query reservations by resource id, user id, status, start time, end time
//     async fn query(
//         &self,
//         request: Request<QueryRequest>,
//     ) -> Result<Response<Self::queryStream>, Status> {
//         todo!()
//     }
//     /// filter reservations, order by reservation id
//     async fn filter(
//         &self,
//         request: Request<FilterRequest>,
//     ) -> Result<Response<FilterResponse>, Status> {
//         todo!()
//     }
//     /// Server streaming response type for the listen method.
//     type listenStream = ReservationStream;
//     /// another system could monitor newly added/confirmed/canceled reservations
//     async fn listen(
//         &self,
//         request: Request<ListenRequest>,
//     ) -> Result<Response<Self::listenStream>, Status> {
//         todo!()
//     }
// }
