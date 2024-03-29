use std::pin::Pin;

use abi::{reservation_service_server::ReservationServiceServer, Config, Reservation};
use futures::Stream;
use reservation::ReservationManager;
use tokio::sync::mpsc;
use tonic::{transport::Server, Status};

mod service;

#[cfg(test)]
pub mod test_utils;
pub struct RsvpService {
    manager: ReservationManager,
}

pub struct TonicReceiverStream<T> {
    inner: mpsc::Receiver<Result<T, abi::Error>>,
}

type ReservationStream = Pin<Box<dyn Stream<Item = Result<Reservation, Status>> + Send>>;

pub async fn start_server(config: &Config) -> Result<(), anyhow::Error> {
    // dbg!(config);
    let addr = format!("{}:{}", config.server.host, config.server.port).parse()?;
    println!("Listening on {addr}");

    let svc = RsvpService::from_config(config).await?;
    let svc = ReservationServiceServer::new(svc);

    Server::builder().add_service(svc).serve(addr).await?;
    Ok(())
}
