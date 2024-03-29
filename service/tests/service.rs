use std::time::Duration;

use abi::{
    reservation_service_client::ReservationServiceClient, Config, ConfirmRequest, FilterRequest,
    FilterResponse, QueryRequest, Reservation, ReservationFilterBuilder, ReservationQueryBuilder,
    ReservationStatus, ReserveRequest,
};
use futures::StreamExt;
use once_cell::sync::Lazy;
use reservation_service::start_server;
use sqlx::QueryBuilder;
use tokio::time;

#[path = "../src/test_utils.rs"]
mod test_utils;
use test_utils::TestConfig;
use tonic::transport::Channel;

#[tokio::test]
async fn grpc_server_should_work() {
    let tconfig = TestConfig::with_server_port(50001);

    let mut client = get_test_client(&tconfig).await;

    // first we make a reservation
    let mut rsvp = Reservation::new_pending(
        "kyros",
        "room",
        "2022-12-26T15:00:00-0700".parse().unwrap(),
        "2022-12-30T12:00:00-0700".parse().unwrap(),
        "book room",
    );

    let ret = client
        .reserve(ReserveRequest::new(rsvp.clone()))
        .await
        .unwrap()
        .into_inner()
        .reservation
        .unwrap();
    rsvp.id = ret.id;
    assert_eq!(ret, rsvp);

    // then we try to make a conflicting reservation
    let rsvp2 = Reservation::new_pending(
        "kyros",
        "room",
        "2022-12-26T15:00:00-0700".parse().unwrap(),
        "2022-12-30T12:00:00-0700".parse().unwrap(),
        "book room",
    );

    let ret = client.reserve(ReserveRequest::new(rsvp2)).await;

    assert!(ret.is_err());

    // then we confirm first reservation
    let ret = client
        .confirm(ConfirmRequest::new(rsvp.id))
        .await
        .unwrap()
        .into_inner()
        .reservation
        .unwrap();
    assert_eq!(ret.status(), ReservationStatus::Confirmed);
}

#[tokio::test]
async fn grpc_query_should_work() {
    let tconfig = TestConfig::with_server_port(50002);
    let mut client = get_test_client(&tconfig).await;
    make_reservations(&mut client, 100).await;

    let query = ReservationQueryBuilder::default()
        .user_id("alice")
        .build()
        .unwrap();
    // query all reservations
    let mut ret = client
        .query(QueryRequest::new(query))
        .await
        .unwrap()
        .into_inner();

    while let Some(Ok(rsvp)) = ret.next().await {
        assert_eq!(rsvp.user_id, "alice");
    }
}

#[tokio::test]
async fn grpc_filter_should_work() {
    let tconfig = TestConfig::with_server_port(50003);
    let mut client = get_test_client(&tconfig).await;
    make_reservations(&mut client, 100).await;

    // then we filter by user
    let filter = ReservationFilterBuilder::default()
        .user_id("alice")
        .status(ReservationStatus::Pending)
        .build()
        .unwrap();
    let FilterResponse {
        pager,
        reservations,
    } = client
        .filter(FilterRequest::new(filter.clone()))
        .await
        .unwrap()
        .into_inner();
    let pager = pager.unwrap();

    assert_eq!(pager.next, Some(filter.page_size + 1));
    assert_eq!(pager.prev, None);

    assert_eq!(reservations.len(), filter.page_size as usize);

    let mut next_filter = filter.clone();
    next_filter.cursor = pager.next;
    // // then we get next page
    // let FilterResponse {
    //     pager,
    //     reservations,
    // } = client
    //     .filter(FilterRequest::new(next_filter.clone()))
    //     .await
    //     .unwrap()
    //     .into_inner();
    // let pager = pager.unwrap();
    // assert_eq!(pager.next, next_filter.cursor + filter.page_size);
    // assert_eq!(pager.prev, next_filter.cursor - 1);

    // assert_eq!(reservations.len(), filter.page_size as usize);
}

async fn get_test_client(tconfig: &TestConfig) -> ReservationServiceClient<Channel> {
    let config = &tconfig.config.clone();

    setup_server(config).await;

    ReservationServiceClient::connect(config.server.url(false))
        .await
        .unwrap()
}

async fn setup_server(config: &Config) {
    let config_cloned = config.clone();
    tokio::spawn(async move {
        start_server(&config_cloned).await.unwrap();
    });
    time::sleep(Duration::from_millis(100)).await;
}

async fn make_reservations(client: &mut ReservationServiceClient<Channel>, count: u32) {
    // we make 100 reservation without conflict
    for i in 0..count {
        let mut rsvp = Reservation::new_pending(
            "alice",
            format!("router-{i}"),
            "2022-12-26T15:00:00-0700".parse().unwrap(),
            "2022-12-30T12:00:00-0700".parse().unwrap(),
            format!("test device reservation - {i}"),
        );

        let ret = client
            .reserve(ReserveRequest::new(rsvp.clone()))
            .await
            .unwrap()
            .into_inner()
            .reservation
            .unwrap();
        rsvp.id = ret.id;

        assert_eq!(ret, rsvp);
    }
}
