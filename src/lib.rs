use chrono::Duration;
use reqwest::{
    Url,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use serde::{Deserialize, Serialize};

mod auth;
mod builders;
mod client;
mod error;

pub use crate::auth::Credentials;
pub use builders::{
    NoteBuilder, TicketClient, TicketCreateBuilder, TicketSearchBuilder, TicketStatus,
    TicketsClient,
};
pub use client::{
    Attachment, Condition, CreateTicketData, Criteria, DetailedTicket, EditTicketData, LogicalOp,
    NameWrapper, Note, NoteData, NoteResponse, Priority, Resolution, SizeInfo, Status, TicketData,
    TicketResponse, TimeEntry, UserInfo,
};
pub use error::{Error, SdpErrorCode};

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq, Hash, Default)]
pub struct UserID(pub String);

#[derive(Clone, Debug)]
pub struct TicketID(pub u64);

#[derive(Clone, Debug)]
pub struct NoteID(pub u64);

impl From<u64> for NoteID {
    fn from(value: u64) -> Self {
        NoteID(value)
    }
}

impl From<NoteID> for u64 {
    fn from(value: NoteID) -> Self {
        value.0
    }
}

impl std::fmt::Display for NoteID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u64> for TicketID {
    fn from(value: u64) -> Self {
        TicketID(value)
    }
}

impl From<TicketID> for u64 {
    fn from(value: TicketID) -> Self {
        value.0
    }
}

impl From<&TicketID> for u64 {
    fn from(value: &TicketID) -> Self {
        value.0
    }
}

impl From<&UserID> for String {
    fn from(value: &UserID) -> Self {
        value.0.clone()
    }
}

impl From<String> for UserID {
    fn from(value: String) -> Self {
        UserID(value)
    }
}

impl From<&str> for UserID {
    fn from(value: &str) -> Self {
        UserID(value.to_string())
    }
}

impl From<u32> for UserID {
    fn from(value: u32) -> Self {
        UserID(value.to_string())
    }
}

impl From<UserID> for u32 {
    fn from(value: UserID) -> Self {
        value.0.parse().unwrap_or_default()
    }
}

impl std::fmt::Display for TicketID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Display for UserID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone)]
pub struct ServiceDesk {
    pub base_url: Url,
    pub credentials: Credentials,
    inner: reqwest::Client,
}

#[derive(Clone, Debug)]
pub enum Security {
    Unsafe,
    NativeTlS,
}

#[derive(Clone, Debug)]
pub struct ServiceDeskOptions {
    user_agent: Option<String>,
    timeout: Option<Duration>,
    security: Option<Security>,
    default_headers: Option<HeaderMap>,
}

static SDP_HEADER: (HeaderName, HeaderValue) = (
    HeaderName::from_static("accept"),
    HeaderValue::from_static("application/vnd.manageengine.sdp.v3+json"),
);

impl Default for ServiceDeskOptions {
    fn default() -> Self {
        ServiceDeskOptions {
            user_agent: Some(String::from("servicedesk-rs/0.1.0")),
            timeout: Some(Duration::seconds(5)),
            security: Some(Security::Unsafe),
            default_headers: Some(HeaderMap::from_iter(vec![SDP_HEADER.clone()])),
        }
    }
}

impl ServiceDesk {
    pub fn new(base_url: Url, credentials: Credentials, options: ServiceDeskOptions) -> Self {
        let mut headers = options.default_headers.unwrap_or_default();

        #[allow(clippy::single_match)]
        match credentials {
            Credentials::Token { ref token } => {
                headers.insert("authtoken", HeaderValue::from_str(token).unwrap());
            }
            _ => {}
        }
        let mut inner = reqwest::ClientBuilder::new()
            .default_headers(headers)
            .user_agent(options.user_agent.unwrap_or_default())
            .timeout(options.timeout.unwrap_or_default().to_std().unwrap());

        if let Some(security) = options.security {
            match security {
                Security::Unsafe => {
                    inner = inner.danger_accept_invalid_certs(true);
                }
                Security::NativeTlS => {
                    // Default behavior, do nothing
                }
            }
        };

        let inner = inner.build().expect("failed to build sdp client");

        ServiceDesk {
            base_url,
            credentials,
            inner,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::client::{EditTicketData, NameWrapper};

    // Fork it and test your setup by setting SDP_TEST_TOKEN and SDP_TEST_URL in a .env file
    pub fn setup() -> ServiceDesk {
        dotenv::dotenv().ok();
        let token = std::env::var("SDP_TEST_TOKEN").expect("SDP_TEST_TOKEN must be set");
        let url = std::env::var("SDP_TEST_URL").expect("SDP_TEST_URL must be set");

        let creds = Credentials::Token { token };

        ServiceDesk::new(
            Url::parse(&url).unwrap(),
            creds,
            ServiceDeskOptions::default(),
        )
    }

    #[tokio::test]
    async fn builder_ticket_get() {
        let sdp = setup();
        let result = sdp.ticket(65997).get().await;
        assert!(result.is_ok());
        let ticket = result.unwrap();
        assert_eq!(ticket.id, "65997");
    }

    #[tokio::test]
    async fn builder_search_open_tickets() {
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
    async fn builder_search_by_alert_id() {
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
    async fn builder_create_ticket() {
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
    async fn builder_add_note() {
        let sdp = setup();
        let result = sdp
            .ticket(65997)
            .add_note("Note added via builder API")
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn builder_note_with_options() {
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
    async fn builder_assign_ticket() {
        let sdp = setup();
        let result = sdp.ticket(250225).assign("Szymon Głuch").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn builder_edit_ticket() {
        let sdp = setup();
        let editdata = EditTicketData {
            subject: "Updated via builder".to_string(),
            description: None,
            requester: Some(NameWrapper {
                name: "GALLUP".to_string(),
            }),
            priority: Some(NameWrapper {
                name: "High".to_string(),
            }),
            udf_fields: None,
        };

        let result = sdp.ticket(250225).edit(&editdata).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn builder_list_notes() {
        let sdp = setup();
        let result = sdp.list_notes(250225, None, None).await;
        assert!(result.is_ok());
        let notes = result.unwrap();
        assert_eq!(
            (notes[0].id.clone(), notes[1].id.clone()),
            ("279486".to_string(), "279666".to_string())
        )
    }

    #[tokio::test]
    async fn builder_get_note() {
        let sdp = setup();
        let result = sdp.get_note(250225, 279486).await;
        assert!(result.is_ok());
        let note = result.unwrap();
        assert_eq!(note.description, "<div>test note<br></div>");
    }

    #[tokio::test]
    async fn builder_create_delete_note() {
        let sdp = setup();
        let create_result = sdp
            .ticket(250225)
            .note()
            .description("Note to be deleted")
            .send()
            .await;
        assert!(create_result.is_ok());
        let created_note = create_result.unwrap();

        let delete_result = sdp
            .delete_note(250225, created_note.id.parse::<u64>().unwrap())
            .await;
        assert!(delete_result.is_ok());
    }
}
