//! Integration tests requiring a real SDP instance.
//!
//! Set SDP_TEST_TOKEN and SDP_TEST_URL environment variables to run.
//! These tests are ignored by default. Run with: cargo test --test integration -- --ignored

use reqwest::Url;
use sdp_request_client::{
    Credentials, EditTicketData, NoteID, Priority, ServiceDesk, ServiceDeskOptions, Status,
    TicketID, UserID, UserInfo,
};

fn setup() -> ServiceDesk {
    dotenv::dotenv().ok();
    let token = std::env::var("SDP_TEST_TOKEN").expect("SDP_TEST_TOKEN must be set");
    let url = std::env::var("SDP_TEST_URL").expect("SDP_TEST_URL must be set");

    let creds = Credentials::Token { token };

    ServiceDesk::new(
        Url::parse(&url).unwrap(),
        creds,
        ServiceDeskOptions::default(),
    )
    .expect("failed to build ServiceDesk client")
}

#[tokio::test]
async fn ticket_get() {
    let sdp = setup();
    let result = sdp.ticket(327847).get().await;
    dbg!(&result);
    assert!(result.is_ok());
    let ticket = result.unwrap();
    assert_eq!(ticket.id, TicketID(285015));
}

#[tokio::test]
async fn ticket_conversations() {
    let sdp = setup();
    let result = sdp.ticket(305892).all_attachment_links().await;
    dbg!(&result);
    assert!(result.is_ok());
}

#[tokio::test]
#[ignore]
async fn search_open_tickets() {
    let sdp = setup();
    let result = sdp
        .tickets()
        .search()
        .open()
        .subject_contains("First")
        .limit(10)
        .fetch()
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
#[ignore]
async fn search_by_alert_id() {
    let sdp = setup();
    let result = sdp
        .tickets()
        .search()
        .field_equals(
            "udf_fields.udf_mline_1202",
            "23433465d4e0ee849a49b994a27a8bbdad726686b73623aebedeef5b69ec1fb2",
        )
        .first()
        .await;
    assert!(result.is_ok());
    let result = result.unwrap();
    assert!(result.is_some());
}

#[tokio::test]
#[ignore]
async fn create_ticket() {
    let sdp = setup();
    let result = sdp
        .tickets()
        .create()
        .subject("[TEST] Test Builder API")
        .description("Created via builder pattern")
        .requester("NETXP")
        .priority("Low")
        .account("SOC - NETXP")
        .template("SOC-with-alert-id")
        .send()
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
#[ignore]
async fn add_note() {
    let sdp = setup();
    let result = sdp
        .ticket(65997)
        .add_note("Note added via builder API")
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
#[ignore]
async fn note_with_options() {
    let sdp = setup();
    let result = sdp
        .ticket(65997)
        .note()
        .description("Note with options via builder")
        .show_to_requester()
        .send()
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
#[ignore]
async fn assign_ticket() {
    let sdp = setup();
    let result = sdp.ticket(250225).assign("Szymon Głuch").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn edit_ticket() {
    let sdp = setup();
    let editdata = EditTicketData {
        subject: "Updated via builder".to_string(),
        status: Status {
            id: 2.to_string(),
            name: "Open".to_string(),
            color: Some("#0066ff".to_string()),
        },
        description: None,
        requester: None,
        priority: Some(Priority::low()),
        udf_fields: None,
    };

    let result = sdp.ticket(250225).edit(&editdata).await;
    assert!(result.is_ok());
}

#[tokio::test]
#[ignore]
async fn list_notes() {
    let sdp = setup();
    let result = sdp.list_notes(250225, None, None).await;
    assert!(result.is_ok());
    let notes = result.unwrap();
    assert_eq!((notes[0].id, notes[1].id), (NoteID(279486), NoteID(279666)))
}

#[tokio::test]
#[ignore]
async fn get_note() {
    let sdp = setup();
    let result = sdp.get_note(250225, 279486).await;
    assert!(result.is_ok());
    let note = result.unwrap();
    assert_eq!(note.description, "<div>test note<br></div>");
}

#[tokio::test]
async fn test_merge() {
    let sdp = setup();
    let result = sdp.ticket(308353).merge(&[TicketID(308345)]).await;
    assert!(result.is_ok());
}

#[tokio::test]
#[ignore]
async fn create_delete_note() {
    let sdp = setup();
    let create_result = sdp
        .ticket(250225)
        .note()
        .description("Note to be deleted")
        .send()
        .await;
    assert!(create_result.is_ok());
    let created_note = create_result.unwrap();

    let delete_result = sdp.delete_note(250225, created_note.id).await;
    assert!(delete_result.is_ok());
}
