use reqwest::Method;
use serde::{Serialize, de::DeserializeOwned};
use serde_aux::field_attributes::deserialize_number_from_string;

#[derive(Debug, Serialize, Deserialize)]
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
#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
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
            let error = response.json::<SdpGenericResponse>().await?;
            tracing::error!(error = ?error, "SDP Error Response");
            return Err(error.response_status.into_error());
        }

        let value = serde_json::to_string(&response.json::<Value>().await?)?;
        let response: R = serde_json::from_str(&value)?;
        tracing::debug!("completed sdp request");
        Ok(response)
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

    /// Edit an existing ticket.
    /// Some of the fields are optional and can be left as None if not being changed.
    /// Some fields might be missing due to SDP API restrictions, like account assignment
    /// to a given ticket being immutable after creation.
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
                        technician: NameWrapper::new(technician_name),
                    },
                },
            )
            .await?;
        Ok(())
    }

    /// Create a new ticket.
    pub async fn create_ticket(&self, data: &CreateTicketData) -> Result<TicketResponse, Error> {
        tracing::info!(subject = %data.subject, "creating ticket");
        let resp = self
            .request_input_data(
                Method::POST,
                "/api/v3/requests",
                &CreateTicketRequest { request: data },
            )
            .await?;
        Ok(resp)
    }

    /// Search for tickets based on specified criteria.
    /// The criteria can be built using the `Criteria` struct.
    /// The default method of querying is not straightforward, [`Criteria`] struct
    /// on the 'root' level contains a single condition, to combine multiple conditions
    /// use the 'children' field with appropriate 'logical_operator'.
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
    pub async fn merge(&self, ticket_id: usize, merge_ids: &[usize]) -> Result<(), Error> {
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
            .map(|id| MergeRequestId { id: id.to_string() })
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

#[derive(Deserialize, Serialize, Debug, Clone)]
pub(crate) struct SearchRequest {
    pub(crate) list_info: ListInfo,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ListInfo {
    pub row_count: u32,
    pub search_criteria: Criteria,
}

/// Criteria structure for building search queries.
/// This structure allows for complex nested criteria using logical operators.
/// The inner field, condition, and value define a single search condition.
/// The children field allows for nesting additional criteria, combined using the specified logical operator.
#[derive(Deserialize, Serialize, Debug, Clone)]
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
#[derive(Deserialize, Serialize, Debug, Clone)]
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
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum LogicalOp {
    #[serde(rename = "AND")]
    And,
    #[serde(rename = "OR")]
    Or,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct TicketSearchResponse {
    pub requests: Vec<DetailedTicket>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Account {
    pub id: String,
    pub name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct DetailedTicketResponse {
    request: DetailedTicket,
    #[serde(skip_serializing)]
    response_status: ResponseStatus,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "request")]
pub struct DetailedTicket {
    pub id: String,
    pub subject: String,
    pub description: Option<String>,
    pub status: Status,
    pub priority: Priority,
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

    pub closure_info: Option<Value>,
    pub site: Option<Value>,
    pub department: Option<Value>,
    pub account: Option<Value>,
}

#[derive(Serialize, Debug)]
struct EditTicketRequest<'a> {
    request: &'a EditTicketData,
}

#[derive(Serialize, Debug)]
pub struct EditTicketData {
    pub subject: String,
    pub description: Option<String>,
    pub requester: Option<NameWrapper>,
    pub priority: Option<NameWrapper>,
    pub udf_fields: Option<Value>,
}

impl From<DetailedTicket> for EditTicketData {
    fn from(value: DetailedTicket) -> Self {
        Self {
            subject: value.subject,
            description: Some(value.description.unwrap_or_default()),
            requester: Some(NameWrapper::new(value.requester.unwrap_or_default().name)),
            priority: Some(value.priority.name.into()),
            udf_fields: value.udf_fields,
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseStatus {
    pub status: String,
    pub status_code: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Status {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Priority {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
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

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SizeInfo {
    pub display_value: String,
    pub value: u64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeEntry {
    pub display_value: String,
    pub value: String,
}

#[derive(Serialize, Debug)]
struct CreateTicketRequest<'a> {
    request: &'a CreateTicketData,
}

#[derive(Serialize, Debug)]
pub struct CreateTicketData {
    pub subject: String,
    pub description: String,
    pub requester: NameWrapper,
    pub priority: NameWrapper,
    // Can't do much here, since these fields seem to be dynamically defined
    // per template at SDP. They need to be explicitly deserialized by the user
    // after we've converted them to plain serde_json::Value.
    pub udf_fields: Value,
    pub account: NameWrapper,
    pub template: NameWrapper,
}

impl Default for CreateTicketData {
    fn default() -> Self {
        CreateTicketData {
            subject: String::new(),
            description: String::new(),
            requester: NameWrapper::new(""),
            priority: NameWrapper::new("Low"),
            udf_fields: Value::Null,
            account: NameWrapper::new(""),
            template: NameWrapper::new(""),
        }
    }
}

#[derive(Serialize, Debug)]
pub struct NameWrapper {
    pub name: String,
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
impl NameWrapper {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
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

#[derive(Serialize, Debug)]
struct CloseTicketRequest {
    request: CloseTicketData,
}

#[derive(Serialize, Debug)]
struct CloseTicketData {
    closure_info: ClosureInfo,
}

#[derive(Serialize, Debug)]
struct ClosureInfo {
    closure_comments: String,
    closure_code: String,
}

#[derive(Serialize, Debug)]
struct AddNoteRequest<'a> {
    note: &'a NoteData,
}

#[derive(Serialize, Debug, Default)]
pub struct NoteData {
    pub mark_first_response: bool,
    pub add_to_linked_requests: bool,
    pub notify_technician: bool,
    pub show_to_requester: bool,
    pub description: String,
}

// Note response structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteResponse {
    pub note: Note,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotesListResponse {
    pub list_info: Option<ListInfoResponse>,
    pub notes: Vec<Note>,
    pub response_status: Vec<ResponseStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListInfoResponse {
    pub has_more_rows: bool,
    pub page: u32,
    pub row_count: u32,
    pub sort_field: String,
    pub sort_order: String,
    pub start_index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
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

#[derive(Serialize, Debug)]
struct NotesListInfo {
    row_count: u32,
    start_index: u32,
}

#[derive(Serialize, Debug)]
struct AssignTicketRequest {
    request: AssignTicketData,
}

#[derive(Serialize, Debug)]
struct AssignTicketData {
    technician: NameWrapper,
}

#[derive(Serialize, Debug)]
struct MergeTicketsRequest {
    merge_requests: Vec<MergeRequestId>,
}

#[derive(Serialize, Debug)]
struct MergeRequestId {
    id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketResponse {
    pub request: TicketData,
    pub response_status: ResponseStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketData {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub id: u64,
    pub subject: String,
    pub description: String,
    pub status: Status,
    pub priority: Priority,
    pub created_time: TimeEntry,
    pub requester: UserInfo,
    pub account: Account,
    pub template: TemplateInfo,
    pub udf_fields: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateInfo {
    pub id: String,
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(data.requester.name.is_empty());
        assert_eq!(data.priority.name, "Low");
        assert!(data.udf_fields.is_null());
        assert!(data.account.name.is_empty());
        assert!(data.template.name.is_empty());
    }
}
