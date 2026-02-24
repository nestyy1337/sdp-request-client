use std::collections::HashSet;

use reqwest::Method;
use serde::{Deserializer, Serialize, Serializer, de::DeserializeOwned, ser::SerializeStruct};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InnerResponseMessage {
    status_code: u32,
    #[serde(rename = "type")]
    type_field: String,
    message: String,
}

/// Generic SDP response status structure
/// Used to parse error responses from the SDP API since SDP uses a non-standard error response format
/// including weird status codes. Partially they are converted to proper HTTP status codes by Error
/// conversion.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdpResponseStatus {
    pub status_code: u32,
    pub messages: Option<Vec<InnerResponseMessage>>,
    pub status: String,
}

impl SdpResponseStatus {
    /// Convert SDP response status to an Error
    pub fn into_error(self) -> Error {
        // Try to get the most specific error code and message from messages array
        if let Some(messages) = &self.messages
            && let Some(msg) = messages.first()
        {
            return Error::from_sdp(msg.status_code, msg.message.clone(), None);
        }
        // Fallback to top-level status code
        Error::from_sdp(self.status_code, self.status, None)
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct SdpGenericResponse {
    response_status: SdpResponseStatus,
}

impl ServiceDesk {
    pub(crate) async fn request_json<T, R>(
        &self,
        method: Method,
        path: &str,
        body: &T,
    ) -> Result<R, Error>
    where
        T: Serialize + ?Sized + std::fmt::Debug,
        R: DeserializeOwned,
    {
        let url = self.base_url.join(path)?;
        let request_builder = self.inner.request(method, url).json(body);

        let response = self.inner.execute(request_builder.build()?).await?;
        if response.error_for_status_ref().is_err() {
            let error = response.json::<SdpGenericResponse>().await?;
            tracing::error!(error = ?error, "SDP Error Response");
            return Err(error.response_status.into_error());
        }

        let parsed = response.json::<R>().await?;
        tracing::debug!("completed sdp request");
        Ok(parsed)
    }

    pub(crate) async fn request_form<T, R>(
        &self,
        method: Method,
        path: &str,
        body: &T,
    ) -> Result<R, Error>
    where
        T: Serialize + ?Sized + std::fmt::Debug,
        R: DeserializeOwned,
    {
        let url = self.base_url.join(path)?;

        let request_builder = self
            .inner
            .request(method, url)
            .form(&[("input_data", serde_json::to_string(body)?)]);

        let response = self.inner.execute(request_builder.build()?).await?;
        if response.error_for_status_ref().is_err() {
            let error = response.json::<SdpGenericResponse>().await?;
            tracing::error!(error = ?error, "SDP Error Response");
            return Err(error.response_status.into_error());
        }

        let parsed = response.json::<R>().await?;
        tracing::debug!("completed sdp request");
        Ok(parsed)
    }

    pub(crate) async fn request_input_data<T, R>(
        &self,
        method: Method,
        path: &str,
        body: &T,
    ) -> Result<R, Error>
    where
        T: Serialize + ?Sized + std::fmt::Debug,
        R: DeserializeOwned,
    {
        let url = self.base_url.join(path)?;

        let request_builder = self
            .inner
            .request(method, url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .query(&[("input_data", serde_json::to_string(body)?)]);

        let response = self.inner.execute(request_builder.build()?).await?;
        if response.error_for_status_ref().is_err() {
            let error = response.json::<SdpGenericResponse>().await?;
            tracing::error!(error = ?error, "SDP Error Response");
            return Err(error.response_status.into_error());
        }
        let result = response.json::<R>().await?;
        tracing::debug!("completed sdp request");
        Ok(result)
    }

    async fn request<T, R>(
        &self,
        method: Method,
        path: &str,
        path_parameter: &T,
    ) -> Result<R, Error>
    where
        T: std::fmt::Display,
        R: DeserializeOwned,
    {
        let url = self
            .base_url
            .join(path)?
            .join(&path_parameter.to_string())?;

        let request_builder = self.inner.request(method, url);
        let response = self.inner.execute(request_builder.build()?).await?;
        if response.error_for_status_ref().is_err() {
            let error = response.json::<SdpGenericResponse>().await.map_err(|e| {
                tracing::error!(error = ?e, "Failed to parse SDP error response");
                Error::from_sdp(
                    500,
                    "Failed to parse SDP error response".to_string(),
                    Some(e.to_string()),
                )
            })?;
            tracing::error!(error = ?error, "SDP Error Response");
            return Err(error.response_status.into_error());
        }

        let response = response.json::<R>().await.map_err(|e| {
            tracing::error!(error = ?e, "Failed to parse SDP response");
            Error::from_sdp(
                500,
                "Failed to parse SDP response".to_string(),
                Some(e.to_string()),
            )
        })?;

        tracing::debug!("completed sdp request");
        Ok(response)
    }

    async fn request_with_path<R>(&self, method: Method, path: &str) -> Result<R, Error>
    where
        R: DeserializeOwned,
    {
        let url = self.base_url.join(path)?;

        let request_builder = self.inner.request(method, url);
        let response = self.inner.execute(request_builder.build()?).await?;
        if response.error_for_status_ref().is_err() {
            let error = response.json::<SdpGenericResponse>().await.map_err(|e| {
                tracing::error!(error = ?e, "Failed to parse SDP error response");
                Error::from_sdp(
                    500,
                    "Failed to parse SDP error response".to_string(),
                    Some(e.to_string()),
                )
            })?;
            tracing::error!(error = ?error, "SDP Error Response");
            return Err(error.response_status.into_error());
        }

        let parsed = response.json::<R>().await?;
        tracing::debug!("completed sdp request");
        Ok(parsed)
    }

    pub async fn ticket_details(
        &self,
        ticket_id: impl Into<TicketID>,
    ) -> Result<DetailedTicket, Error> {
        let ticket_id = ticket_id.into();
        tracing::info!(ticket_id = %ticket_id, "fetching ticket details");
        let resp: DetailedTicketResponse = self
            .request(Method::GET, "/api/v3/requests/", &ticket_id)
            .await?;
        Ok(resp.request)
    }

    pub async fn get_conversations(&self, ticket_id: impl Into<TicketID>) -> Result<Value, Error> {
        let ticket_id = ticket_id.into();
        tracing::info!(ticket_id = %ticket_id, "fetching ticket details");
        let path = format!("/api/v3/requests/{}/conversations", &ticket_id);
        let resp: Value = self.request_with_path(Method::GET, &path).await?;
        Ok(resp)
    }

    async fn get_conversations_typed(
        &self,
        ticket_id: impl Into<TicketID>,
    ) -> Result<ConversationsResponse, Error> {
        let ticket_id = ticket_id.into();
        tracing::info!(ticket_id = %ticket_id, "fetching ticket conversations");
        let path = format!("/api/v3/requests/{}/conversations", &ticket_id);
        self.request_with_path(Method::GET, &path).await
    }

    pub async fn get_conversation_content(&self, content_url: &str) -> Result<Value, Error> {
        tracing::info!(content_url = %content_url, "fetching conversation content");
        let resp: Value = self.request_with_path(Method::GET, content_url).await?;
        Ok(resp)
    }

    async fn get_conversation_attachments(
        &self,
        content_url: &str,
    ) -> Result<Vec<Attachment>, Error> {
        tracing::info!(content_url = %content_url, "fetching conversation attachments");
        let resp: Value = self.request_with_path(Method::GET, content_url).await?;
        let attachment: Vec<Attachment> = serde_json::from_value(
            resp.get("notification")
                .unwrap_or_default()
                .get("attachments")
                .cloned()
                .unwrap_or_default(),
        )?;
        Ok(attachment)
    }

    pub async fn get_conversation_attachment_urls(
        &self,
        ticket_id: impl Into<TicketID>,
    ) -> Result<Vec<String>, Error> {
        let conversations = self.get_conversations_typed(ticket_id).await?;
        let mut links = HashSet::new();

        for conversation in conversations.conversations {
            if !conversation.has_attachments {
                continue;
            }

            let Some(content_url) = conversation.content_url.as_deref() else {
                continue;
            };

            let attachments = self.get_conversation_attachments(content_url).await?;
            for attachment in attachments {
                links.insert(normalize_attachment_url(
                    &self.base_url,
                    &attachment.content_url,
                )?);
            }
        }

        let mut links: Vec<String> = links.into_iter().collect();
        links.sort();
        Ok(links)
    }

    pub async fn download_attachment(&self, attachment_url: &str) -> Result<Vec<u8>, Error> {
        tracing::info!(attachment_url = %attachment_url, "downloading attachment");
        let url = self.base_url.join(attachment_url)?;
        let response = self.inner.get(url).send().await?;
        if response.error_for_status_ref().is_err() {
            let error = response.json::<SdpGenericResponse>().await.map_err(|e| {
                tracing::error!(error = ?e, "Failed to parse SDP error response");
                Error::from_sdp(
                    500,
                    "Failed to parse SDP error response".to_string(),
                    Some(e.to_string()),
                )
            })?;
            tracing::error!(error = ?error, "SDP Error Response");
            return Err(error.response_status.into_error());
        }
        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    /// Edit an existing ticket.
    ///
    /// # Important
    /// Read `EditTicketData` documentation for details on how the editing works and how to use it.
    pub async fn edit(
        &self,
        ticket_id: impl Into<TicketID>,
        data: &EditTicketData,
    ) -> Result<(), Error> {
        let ticket_id = ticket_id.into();
        tracing::info!(ticket_id = %ticket_id, "editing ticket");
        let _: SdpGenericResponse = self
            .request_input_data(
                Method::PUT,
                &format!("/api/v3/requests/{}", ticket_id),
                &EditTicketRequest { request: data },
            )
            .await?;
        Ok(())
    }

    /// Add a note to a ticket (creates a new note).
    pub async fn add_note(
        &self,
        ticket_id: impl Into<TicketID>,
        note: &NoteData,
    ) -> Result<Note, Error> {
        let ticket_id = ticket_id.into();
        tracing::info!(ticket_id = %ticket_id, "adding note");
        let resp: NoteResponse = self
            .request_input_data(
                Method::POST,
                &format!("/api/v3/requests/{}/notes", ticket_id),
                &AddNoteRequest { note },
            )
            .await?;
        Ok(resp.note)
    }

    /// Get a specific note from a ticket.
    pub async fn get_note(
        &self,
        ticket_id: impl Into<TicketID>,
        note_id: impl Into<NoteID>,
    ) -> Result<Note, Error> {
        let ticket_id = ticket_id.into();
        let note_id = note_id.into();
        tracing::info!(ticket_id = %ticket_id, note_id = %note_id, "fetching note");
        let url = format!("/api/v3/requests/{}/notes/{}", ticket_id, note_id);
        let resp: NoteResponse = self.request(Method::GET, &url, &"").await?;
        Ok(resp.note)
    }

    /// List all notes for a ticket.
    pub async fn list_notes(
        &self,
        ticket_id: impl Into<TicketID>,
        row_count: Option<u32>,
        start_index: Option<u32>,
    ) -> Result<Vec<Note>, Error> {
        let ticket_id = ticket_id.into();
        tracing::info!(ticket_id = %ticket_id, "listing notes");
        let body = ListNotesRequest {
            list_info: NotesListInfo {
                row_count: row_count.unwrap_or(100),
                start_index: start_index.unwrap_or(1),
            },
        };
        let resp: Value = self
            .request_input_data(
                Method::GET,
                &format!("/api/v3/requests/{}/notes", ticket_id),
                &body,
            )
            .await?;
        let resp: NotesListResponse = serde_json::from_value(resp)?;
        Ok(resp.notes)
    }

    /// Edit an existing note.
    pub async fn edit_note(
        &self,
        ticket_id: impl Into<TicketID>,
        note_id: impl Into<NoteID>,
        note: &NoteData,
    ) -> Result<Note, Error> {
        let ticket_id = ticket_id.into();
        let note_id = note_id.into();
        tracing::info!(ticket_id = %ticket_id, note_id = %note_id, "editing note");
        let resp: NoteResponse = self
            .request_input_data(
                Method::PUT,
                &format!("/api/v3/requests/{}/notes/{}", ticket_id, note_id),
                &EditNoteRequest { request_note: note },
            )
            .await?;
        Ok(resp.note)
    }

    /// Delete a note from a ticket.
    pub async fn delete_note(
        &self,
        ticket_id: impl Into<TicketID>,
        note_id: impl Into<NoteID>,
    ) -> Result<(), Error> {
        let ticket_id = ticket_id.into();
        let note_id = note_id.into();
        tracing::info!(ticket_id = %ticket_id, note_id = %note_id, "deleting note");
        let _: SdpGenericResponse = self
            .request(
                Method::DELETE,
                &format!("/api/v3/requests/{}/notes/{}", ticket_id, note_id),
                &"",
            )
            .await?;
        Ok(())
    }

    /// Assign a ticket to a technician.
    pub async fn assign_ticket(
        &self,
        ticket_id: impl Into<TicketID>,
        technician_name: &str,
    ) -> Result<(), Error> {
        let ticket_id = ticket_id.into();
        tracing::info!(ticket_id = %ticket_id, technician = %technician_name, "assigning ticket");
        let _: SdpGenericResponse = self
            .request_input_data(
                Method::PUT,
                &format!("/api/v3/requests/{}/assign", ticket_id),
                &AssignTicketRequest {
                    request: AssignTicketData {
                        technician: technician_name.to_string(),
                    },
                },
            )
            .await?;
        Ok(())
    }

    /// Create a new ticket.
    pub async fn create_ticket(&self, data: &CreateTicketData) -> Result<TicketData, Error> {
        tracing::info!(subject = %data.subject, "creating ticket");
        let resp: TicketResponse = self
            .request_input_data(
                Method::POST,
                "/api/v3/requests",
                &CreateTicketRequest { request: data },
            )
            .await?;
        Ok(resp.request)
    }

    /// Search for tickets based on specified criteria.
    ///
    /// The criteria can be built using the `Criteria` struct.
    /// The default method of querying is not straightforward,
    /// [`Criteria`] struct on the 'root' level contains a single condition, to combine multiple conditions
    /// use the 'children' field with appropriate `LogicalOp`.
    pub async fn search_tickets(&self, criteria: Criteria) -> Result<Vec<DetailedTicket>, Error> {
        tracing::info!("searching tickets");
        let resp = self
            .request_input_data(
                Method::GET,
                "/api/v3/requests",
                &SearchRequest {
                    list_info: ListInfo {
                        row_count: 100,
                        search_criteria: criteria,
                    },
                },
            )
            .await?;

        let ticket_response: TicketSearchResponse = serde_json::from_value(resp)?;

        Ok(ticket_response.requests)
    }

    /// Close a ticket with closure comments.
    pub async fn close_ticket(
        &self,
        ticket_id: impl Into<TicketID>,
        closure_comments: &str,
    ) -> Result<(), Error> {
        let ticket_id = ticket_id.into();
        tracing::info!(ticket_id = %ticket_id, "closing ticket");
        let _: SdpGenericResponse = self
            .request_json(
                Method::PUT,
                &format!("/api/v3/requests/{}/close", ticket_id),
                &CloseTicketRequest {
                    request: CloseTicketData {
                        closure_info: ClosureInfo {
                            closure_comments: closure_comments.to_string(),
                            closure_code: "Closed".to_string(),
                        },
                    },
                },
            )
            .await?;
        Ok(())
    }

    /// Merge multiple tickets into a single ticket.
    /// Key point to note is that the maximum number of tickets that can be merged at once is 49 +
    /// 1 (the target ticket), so the `merge_ids` slice must not exceed 49 IDs.
    pub async fn merge(
        &self,
        ticket_id: impl Into<TicketID>,
        merge_ids: &[TicketID],
    ) -> Result<(), Error> {
        let ticket_id = ticket_id.into();
        tracing::info!(ticket_id = %ticket_id, count = merge_ids.len(), "merging tickets");
        if merge_ids.len() > 49 {
            tracing::warn!("attempted to merge more than 49 tickets");
            return Err(Error::from_sdp(
                400,
                "Cannot merge more than 49 tickets at once".to_string(),
                None,
            ));
        }
        let merge_requests: Vec<MergeRequestId> = merge_ids
            .iter()
            .map(|id| MergeRequestId {
                id: id.0.to_string(),
            })
            .collect();

        let _: SdpGenericResponse = self
            .request_form(
                Method::PUT,
                &format!("/api/v3/requests/{}/merge_requests", ticket_id),
                &MergeTicketsRequest { merge_requests },
            )
            .await?;
        Ok(())
    }
}

use serde::Deserialize;
use serde_json::Value;

use crate::{NoteID, ServiceDesk, TicketID, UserID, error::Error};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub(crate) struct SearchRequest {
    pub(crate) list_info: ListInfo,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct ListInfo {
    pub row_count: u32,
    pub search_criteria: Criteria,
}

/// Criteria structure for building search queries.
/// This structure allows for complex nested criteria using logical operators.
/// The inner field, condition, and value define a single search condition.
/// The children field allows for nesting additional criteria, combined using the specified logical operator.
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Criteria {
    pub field: String,
    pub condition: Condition,
    pub value: Value,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Criteria>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub logical_operator: Option<LogicalOp>,
}

impl Default for Criteria {
    fn default() -> Self {
        Criteria {
            field: String::new(),
            condition: Condition::Is,
            value: Value::Null,
            children: vec![],
            logical_operator: None,
        }
    }
}

/// Condition enum for specifying search conditions in criteria.
/// Used in the Criteria struct to define how to compare field values.
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Condition {
    #[serde(rename = "is")]
    Is,
    #[serde(rename = "greater than")]
    GreaterThan,
    #[serde(rename = "lesser than")]
    LesserThan,
    #[serde(rename = "contains")]
    Contains,
}

/// Logical operators for combining multiple criteria.
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub enum LogicalOp {
    #[serde(rename = "AND")]
    And,
    #[serde(rename = "OR")]
    Or,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct TicketSearchResponse {
    pub requests: Vec<DetailedTicket>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct DetailedTicketResponse {
    request: DetailedTicket,
    #[serde(skip_serializing)]
    response_status: ResponseStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "request")]
pub struct DetailedTicket {
    pub id: TicketID,
    pub subject: String,
    pub description: Option<String>,
    pub status: Status,
    pub priority: Option<Priority>,
    pub requester: Option<UserInfo>,
    pub technician: Option<UserInfo>,
    #[serde(skip_serializing)]
    pub created_by: UserInfo,
    pub created_time: TimeEntry,
    pub resolution: Option<Resolution>,
    pub due_by_time: Option<TimeEntry>,
    pub resolved_time: Option<TimeEntry>,
    pub completed_time: Option<TimeEntry>,
    pub udf_fields: Option<Value>,
    pub attachments: Option<Vec<Attachment>>,
    pub closure_info: Option<Value>,
    pub site: Option<Value>,
    pub department: Option<Value>,
    pub account: Option<Value>,
}

#[derive(Serialize, Debug)]
struct EditTicketRequest<'a> {
    request: &'a EditTicketData,
}

/// Data structure for editing a ticket.
/// Contains fields that WILL be updated on the associated ticket.
/// For some reason SDP does not provide simple API to patch a single attribute of a ticket,
/// instead it requires sending a PUT that will replace all of the fields even None ones,
/// which will be treated as empty values and overwrite existing data.
///
/// To conveniently use this API I'd recommend to use `From<DetailedTicket>` implementation for this struct.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct EditTicketData {
    pub subject: String,
    pub status: Status,
    pub description: Option<String>,
    #[serde(
        serialize_with = "serialize_optional_name_object",
        deserialize_with = "deserialize_optional_name_object"
    )]
    pub requester: Option<String>,
    #[serde(
        serialize_with = "serialize_optional_name_object",
        deserialize_with = "deserialize_optional_name_object"
    )]
    pub priority: Option<String>,
    /// Dynamically defined template fields
    pub udf_fields: Option<Value>,
}

impl From<DetailedTicket> for EditTicketData {
    fn from(value: DetailedTicket) -> Self {
        let priority = value.priority.as_ref().map(|p| p.name.clone());
        Self {
            subject: value.subject,
            status: value.status,
            description: Some(value.description.unwrap_or_default()),
            requester: Some(value.requester.unwrap_or_default().name),
            priority,
            udf_fields: value.udf_fields,
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ResponseStatus {
    pub(crate) status: String,
    pub(crate) status_code: i64,
}

pub const STATUS_ID_OPEN: u64 = 2;
pub const STATUS_ID_ASSIGNED: u64 = 5;
pub const STATUS_ID_CANCELLED: u64 = 7;
pub const STATUS_ID_CLOSED: u64 = 1;
pub const STATUS_ID_IN_PROGRESS: u64 = 6;
pub const STATUS_ID_ONHOLD: u64 = 3;
pub const STATUS_ID_RESOLVED: u64 = 4;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Status {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
}

impl Status {
    pub fn open() -> Self {
        Status {
            id: STATUS_ID_OPEN.to_string(),
            name: "Open".to_string(),
            color: Some("#0066ff".to_string()),
        }
    }

    pub fn assigned() -> Self {
        Status {
            id: STATUS_ID_ASSIGNED.to_string(),
            name: "Assigned".to_string(),
            // blue
            color: Some("#0000ff".to_string()),
        }
    }

    pub fn cancelled() -> Self {
        Status {
            id: STATUS_ID_CANCELLED.to_string(),
            name: "Cancelled".to_string(),
            // grey
            color: Some("#999999".to_string()),
        }
    }

    pub fn closed() -> Self {
        Status {
            id: STATUS_ID_CLOSED.to_string(),
            name: "Closed".to_string(),
            color: Some("#006600".to_string()),
        }
    }

    pub fn in_progress() -> Self {
        Status {
            id: STATUS_ID_IN_PROGRESS.to_string(),
            name: "In Progress".to_string(),
            color: Some("#00ffcc".to_string()),
        }
    }

    pub fn onhold() -> Self {
        Status {
            id: STATUS_ID_ONHOLD.to_string(),
            name: "On Hold".to_string(),
            color: Some("#ff0000".to_string()),
        }
    }

    pub fn resolved() -> Self {
        Status {
            id: STATUS_ID_RESOLVED.to_string(),
            name: "Resolved".to_string(),
            color: Some("#00ff66".to_string()),
        }
    }
}

/// Priority structure representing the priority of a ticket in SDP.
/// Contains an ID, name, and an optional color for visual representation.
///
/// 'Not specified' priority is represented by None, which is the default value for the Priority struct.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Priority {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
}

pub const PRIORITY_ID_LOW: u64 = 1;
pub const PRIORITY_ID_MEDIUM: u64 = 3;
pub const PRIORITY_ID_HIGH: u64 = 4;
pub const PRIORITY_ID_CRITICAL: u64 = 301;

// priority: Some(
//     Priority {
//         id: "1",
//         name: "Low",
//         color: Some(
//             "#288251",
//         ),
//     },
//
// priority: Some(
//     Priority {
//         id: "3",
//         name: "Medium",
//         color: Some(
//             "#efb116",
//         ),
//     },
// ),
//
//     Priority {
//         priority: Some(
//         id: "4",
//         name: "High",
//         color: Some(
//             "#ff5e00",
//         ),
//     },
// ),
//
// priority: Some(
//     Priority {
//         id: "301",
//         name: "Critical",
//         color: Some(
//             "#8b0808",
//         ),
//     },
// ),
impl Priority {
    pub fn low() -> Self {
        Priority {
            id: PRIORITY_ID_LOW.to_string(),
            name: "Low".to_string(),
            color: Some("#288251".to_string()),
        }
    }

    pub fn medium() -> Self {
        Priority {
            id: PRIORITY_ID_MEDIUM.to_string(),
            name: "Medium".to_string(),
            color: Some("#efb116".to_string()),
        }
    }

    pub fn high() -> Self {
        Priority {
            id: PRIORITY_ID_HIGH.to_string(),
            name: "High".to_string(),
            color: Some("#ff5e00".to_string()),
        }
    }

    /// Suspiciously high internal ID, might be specific to our SDP instance.
    /// Please verify on your end if this ID is correct for the Critical priority, or if it needs to be adjusted.
    pub fn critical() -> Self {
        Priority {
            id: PRIORITY_ID_CRITICAL.to_string(),
            name: "Critical".to_string(),
            color: Some("#8b0808".to_string()),
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: UserID,
    pub name: String,
    pub email_id: Option<String>,
    pub account: Option<Value>,
    pub department: Option<Value>,
    #[serde(default)]
    pub is_vipuser: bool,
    pub mobile: Option<String>,
    pub org_user_status: Option<String>,
    pub phone: Option<String>,
    #[serde(skip_serializing)]
    pub profile_pic: Option<Value>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resolution {
    pub content: Option<String>,
    pub submitted_by: Option<UserInfo>,
    pub submitted_on: Option<TimeEntry>,
    pub resolution_attachments: Option<Vec<Attachment>>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attachment {
    pub id: String,
    pub name: String,
    pub content_url: String,
    pub content_type: Option<String>,
    pub description: Option<String>,
    pub module: Option<String>,
    pub size: Option<SizeInfo>,
    pub attached_by: Option<UserInfo>,
    pub attached_on: Option<TimeEntry>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SizeInfo {
    pub display_value: String,
    pub value: u64,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeEntry {
    pub display_value: String,
    pub value: String,
}

#[derive(Serialize, Debug)]
struct CreateTicketRequest<'a> {
    request: &'a CreateTicketData,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct CreateTicketData {
    pub subject: String,
    pub description: String,
    #[serde(
        serialize_with = "serialize_name_object",
        deserialize_with = "deserialize_name_object"
    )]
    pub requester: String,
    #[serde(
        serialize_with = "serialize_name_object",
        deserialize_with = "deserialize_name_object"
    )]
    pub priority: String,
    // Can't do much here, since these fields seem to be dynamically defined
    // per template at SDP. They need to be explicitly deserialized by the user
    // after we've converted them to plain serde_json::Value.
    pub udf_fields: Value,
    #[serde(
        serialize_with = "serialize_name_object",
        deserialize_with = "deserialize_name_object"
    )]
    pub account: String,
    #[serde(
        serialize_with = "serialize_name_object",
        deserialize_with = "deserialize_name_object"
    )]
    pub template: String,
}

impl Default for CreateTicketData {
    fn default() -> Self {
        CreateTicketData {
            subject: String::new(),
            description: String::new(),
            requester: String::new(),
            priority: "Low".to_string(),
            udf_fields: Value::Null,
            account: String::new(),
            template: String::new(),
        }
    }
}

pub(crate) fn deserialize_name_object<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct NameObject {
        name: String,
    }

    Ok(NameObject::deserialize(deserializer)?.name)
}

pub(crate) fn deserialize_optional_name_object<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct NameObject {
        name: String,
    }

    Ok(Option::<NameObject>::deserialize(deserializer)?.map(|name| name.name))
}

pub(crate) fn serialize_name_object<S>(name: &String, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut s = serializer.serialize_struct("NameWrapper", 1)?;
    s.serialize_field("name", name)?;
    s.end()
}

pub(crate) fn serialize_optional_name_object<S>(
    maybe_name: &Option<String>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match maybe_name {
        Some(name) => serialize_name_object(name, serializer),
        None => serializer.serialize_none(),
    }
}

#[allow(dead_code)]
#[derive(Serialize, Debug, PartialEq, Eq)]
pub(crate) struct NameWrapper {
    pub(crate) name: String,
}

impl From<&str> for NameWrapper {
    fn from(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

impl From<String> for NameWrapper {
    fn from(name: String) -> Self {
        Self { name }
    }
}

impl std::ops::Deref for NameWrapper {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.name
    }
}

impl std::ops::DerefMut for NameWrapper {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.name
    }
}

#[derive(Serialize, Debug, PartialEq, Eq)]
struct CloseTicketRequest {
    request: CloseTicketData,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
struct CloseTicketData {
    closure_info: ClosureInfo,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
struct ClosureInfo {
    closure_comments: String,
    closure_code: String,
}

#[derive(Serialize, Debug)]
struct AddNoteRequest<'a> {
    note: &'a NoteData,
}

#[derive(Serialize, Debug, Default, PartialEq, Eq)]
pub struct NoteData {
    pub mark_first_response: bool,
    pub add_to_linked_requests: bool,
    pub notify_technician: bool,
    pub show_to_requester: bool,
    pub description: String,
}

// Note response structures
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct NoteResponse {
    pub(crate) note: Note,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NotesListResponse {
    pub list_info: Option<ListInfoResponse>,
    pub notes: Vec<Note>,
    pub response_status: Vec<ResponseStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListInfoResponse {
    pub has_more_rows: bool,
    pub page: u32,
    pub row_count: u32,
    pub sort_field: String,
    pub sort_order: String,
    pub start_index: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Note {
    pub id: NoteID,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub show_to_requester: bool,
    #[serde(default)]
    pub mark_first_response: bool,
    #[serde(default)]
    pub notify_technician: bool,
    #[serde(default)]
    pub add_to_linked_requests: bool,
    pub created_time: Option<TimeEntry>,
    pub created_by: Option<UserInfo>,
    pub last_updated_time: Option<TimeEntry>,
}

#[derive(Serialize, Debug)]
struct EditNoteRequest<'a> {
    request_note: &'a NoteData,
}

#[derive(Serialize, Debug)]
struct ListNotesRequest {
    list_info: NotesListInfo,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
struct ConversationsResponse {
    #[serde(default)]
    conversations: Vec<ConversationSummary>,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
struct ConversationSummary {
    #[serde(default)]
    has_attachments: bool,
    #[serde(default)]
    content_url: Option<String>,
}

fn normalize_attachment_url(base_url: &reqwest::Url, value: &str) -> Result<String, Error> {
    Ok(base_url.join(value)?.to_string())
}

#[derive(Serialize, Debug, PartialEq, Eq)]
struct NotesListInfo {
    row_count: u32,
    start_index: u32,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
struct AssignTicketRequest {
    request: AssignTicketData,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
struct AssignTicketData {
    #[serde(
        serialize_with = "serialize_name_object",
        deserialize_with = "deserialize_name_object"
    )]
    technician: String,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
struct MergeTicketsRequest {
    merge_requests: Vec<MergeRequestId>,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
struct MergeRequestId {
    id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct TicketResponse {
    pub(crate) request: TicketData,
    pub(crate) response_status: ResponseStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TicketData {
    pub id: TicketID,
    pub subject: String,
    pub description: Option<String>,
    pub status: Status,
    pub priority: Option<Priority>,
    pub created_time: TimeEntry,
    pub requester: Option<UserInfo>,
    pub account: Account,
    pub template: TemplateInfo,
    pub udf_fields: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateInfo {
    pub id: String,
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn criteria_default() {
        let criteria = Criteria::default();
        assert!(criteria.field.is_empty());
        assert!(matches!(criteria.condition, Condition::Is));
        assert!(criteria.value.is_null());
        assert!(criteria.children.is_empty());
        assert!(criteria.logical_operator.is_none());
    }

    #[test]
    fn create_ticket_data_default() {
        let data = CreateTicketData::default();
        assert!(data.subject.is_empty());
        assert!(data.description.is_empty());
        assert!(data.requester.is_empty());
        assert_eq!(data.priority, "Low");
        assert!(data.udf_fields.is_null());
        assert!(data.account.is_empty());
        assert!(data.template.is_empty());
    }

    #[test]
    fn create_ticket_data_serializes_name_fields_as_objects() {
        let data = CreateTicketData {
            subject: "test".to_string(),
            description: "body".to_string(),
            requester: "NETXP".to_string(),
            priority: "High".to_string(),
            udf_fields: json!({}),
            account: "SOC".to_string(),
            template: "SOC-with-alert-id".to_string(),
        };

        let serialized = serde_json::to_value(&data).unwrap();

        assert_eq!(serialized["requester"], json!({ "name": "NETXP" }));
        assert_eq!(serialized["priority"], json!({ "name": "High" }));
        assert_eq!(serialized["account"], json!({ "name": "SOC" }));
        assert_eq!(
            serialized["template"],
            json!({ "name": "SOC-with-alert-id" })
        );
    }

    #[test]
    fn edit_ticket_data_serializes_optional_name_fields_as_objects() {
        let data = EditTicketData {
            subject: "test".to_string(),
            status: Status {
                id: "1".to_string(),
                name: "Open".to_string(),
                color: None,
            },
            description: None,
            requester: Some("NETXP".to_string()),
            priority: Some("High".to_string()),
            udf_fields: None,
        };

        let serialized = serde_json::to_value(&data).unwrap();

        assert_eq!(serialized["requester"], json!({ "name": "NETXP" }));
        assert_eq!(serialized["priority"], json!({ "name": "High" }));
    }

    #[test]
    fn deserialize_name_helpers_extract_name_values() {
        let mut name_de = serde_json::Deserializer::from_str(r#"{ "name": "High" }"#);
        let name = deserialize_name_object(&mut name_de).unwrap();
        assert_eq!(name, "High");

        let mut some_de = serde_json::Deserializer::from_str(r#"{ "name": "NETXP" }"#);
        let maybe_name = deserialize_optional_name_object(&mut some_de).unwrap();
        assert_eq!(maybe_name, Some("NETXP".to_string()));

        let mut none_de = serde_json::Deserializer::from_str("null");
        let none_name = deserialize_optional_name_object(&mut none_de).unwrap();
        assert_eq!(none_name, None);
    }
}
