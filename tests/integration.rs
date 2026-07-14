//! Integration tests requiring a real SDP instance.
//!
//! Set SDP_TEST_TOKEN and SDP_TEST_URL environment variables to run.
//! These tests are ignored by default. Run with: cargo test --test integration -- --ignored

use std::path::Path;

use reqwest::Url;
use sdp_request_client::{
    Credentials, EditTicketData, Error, NoteID, Priority, ServiceDesk, ServiceDeskOptions, Status,
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
#[ignore]
async fn merged_ticket_get_returns_parent() {
    let sdp = setup();
    let result = sdp.ticket(583550).get().await;

    assert!(matches!(
        result,
        Err(Error::RequestMerged {
            parent_request_id: TicketID(583415),
            ..
        })
    ));
}

#[tokio::test]
#[ignore]
async fn add_attachment() {
    let sdp = setup();
    let path = Path::new("/home/szymon/Downloads/BLACK.PNG");
    let result = sdp.ticket(585627).add_attachment(path).await;
    dbg!(&result);
    assert!(result.is_ok());
}

#[tokio::test]
#[ignore]
async fn worklog_add() {
    let sdp = setup();
    let result = sdp
        .ticket(583588)
        .worklog()
        .description("Worklog added via builder")
        .owner(UserInfo {
            id: UserID("1541".to_string()),
            name: "test".to_string(),
            email_id: None,
            account: None,
            department: None,
            is_vipuser: false,
            mobile: None,
            org_user_status: None,
            phone: None,
            profile_pic: None,
        })
        .mark_first_response()
        .send()
        .await;
    dbg!(&result);
    assert!(result.is_ok());
}

#[tokio::test]
#[ignore]
async fn ticket_conversations_all() {
    let sdp = setup();
    let result = sdp.ticket(575493).conversations().await;
    dbg!(&result);
    assert!(result.is_ok());
}

#[tokio::test]
#[ignore]
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
        .priority(Priority::low())
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
#[ignore]
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
#[ignore]
async fn repeated_merge_is_rejected_but_relationship_exists() {
    let sdp = setup();
    let result = sdp.ticket(583415).merge(&[TicketID(583550)]).await;

    assert!(matches!(
        result,
        Err(Error::Forbidden(message))
            if message == "Operation not supported across accounts."
    ));

    let merged = sdp.ticket(583415).merged_ticket_ids().await.unwrap();
    assert!(merged.contains(&TicketID(583550)));
}

#[tokio::test]
#[ignore]
async fn merged_ticket_ids() {
    let sdp = setup();
    let merged = sdp.ticket(575493).merged_ticket_ids().await.unwrap();
    assert_eq!(merged, vec![TicketID(575483)]);
}

#[tokio::test]
#[ignore]
async fn create_delete_note() {
    let sdp = setup();
    let note = sdp
        .ticket(250225)
        .note()
        .description("Note to be deleted")
        .build();
    let create_result = sdp.add_note(250225, &note).await;
    assert!(create_result.is_ok());
    let created_note = create_result.unwrap();

    let delete_result = sdp.delete_note(250225, created_note.id).await;
    assert!(delete_result.is_ok());
}
