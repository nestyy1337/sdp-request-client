//! Fluent builders for SDP API operations.
//!
//! # Example
//! ```no_run
//! # use sdp_request_client::{ServiceDesk, ServiceDeskOptions, Credentials};
//! # use reqwest::Url;
//! # async fn example() -> Result<(), sdp_request_client::Error> {
//! # let client = ServiceDesk::new(Url::parse("https://sdp.example.com").unwrap(), Credentials::Token { token: "".into() }, ServiceDeskOptions::default());
//! // Search for open tickets (default limit: 100)
//! let tickets = client.tickets()
//!     .search()
//!     .open()
//!     .limit(50)
//!     .fetch()
//!     .await?;
//!
//! // Create a ticket (subject and requester required, priority defaults to "Low")
//! let ticket = client.tickets()
//!     .create()
//!     .subject("[CLIENT] Alert Name")
//!     .description("Alert details...")
//!     .priority("High")
//!     .requester("CLIENT")
//!     .send()
//!     .await?;
//!
//! // Single ticket operations
//! client.ticket(12345).add_note("Resolved by automation").await?;
//! client.ticket(12345).close("Closed by automation").await?;
//! # Ok(())
//! # }
//! ```

use chrono::{DateTime, Local};
use reqwest::Method;
use serde_json::Value;

use crate::{
    ServiceDesk, TicketID,
    client::{
        Condition, CreateTicketData, Criteria, DetailedTicket, EditTicketData, ListInfo, LogicalOp,
        NameWrapper, Note, NoteData, SearchRequest, TicketResponse, TicketSearchResponse,
    },
    error::Error,
};

/// Client for ticket collection operations (search, create, delete, update).
pub struct TicketsClient<'a> {
    pub(crate) client: &'a ServiceDesk,
}

impl<'a> TicketsClient<'a> {
    /// Start building a ticket search query. Default limit is 100.
    pub fn search(self) -> TicketSearchBuilder<'a> {
        TicketSearchBuilder {
            client: self.client,
            root_criteria: None,
            children: vec![],
            row_count: 100,
        }
    }

    /// Start building a new ticket.
    pub fn create(self) -> TicketCreateBuilder<'a> {
        TicketCreateBuilder {
            client: self.client,
            subject: None,
            description: None,
            requester: None,
            priority: "Low".to_string(),
            account: None,
            template: None,
            udf_fields: None,
        }
    }
}

/// Client for single ticket operations (get, close, assign, notes, merge).
pub struct TicketClient<'a> {
    pub(crate) client: &'a ServiceDesk,
    pub(crate) id: TicketID,
}

impl<'a> TicketClient<'a> {
    /// Get full ticket details.
    pub async fn get(self) -> Result<DetailedTicket, Error> {
        self.client.ticket_details(self.id).await
    }

    /// Close the ticket with a comment.
    pub async fn close(self, comment: &str) -> Result<(), Error> {
        self.client.close_ticket(self.id, comment).await
    }

    /// Assign the ticket to a technician.
    pub async fn assign(self, technician: &str) -> Result<(), Error> {
        self.client.assign_ticket(self.id, technician).await
    }

    pub async fn conversations(self) -> Result<Value, Error> {
        self.client.get_conversations(self.id).await
    }

    pub async fn conversation_content(self, content_url: &str) -> Result<Value, Error> {
        self.client.get_conversation_content(content_url).await
    }

    /// Get all attachment links for the ticket, including conversation attachments
    /// including attachments from merged tickets.
    pub async fn all_attachment_links(self) -> Result<Vec<String>, Error> {
        let ticket = self.client.ticket(self.id.clone()).get().await?;
        let mut links = Vec::new();
        for attachment in ticket.attachments {
            links.push(format!(
                "{}{}",
                self.client.base_url, attachment.content_url
            ));
        }
        if let Ok(attachments) = self.client.get_conversation_attachment_urls(self.id).await {
            for url in attachments {
                links.push(format!("{}{}", self.client.base_url, url));
            }
        }
        Ok(links)
    }

    /// Add a note to the ticket with default settings.
    pub async fn add_note(self, description: &str) -> Result<Note, Error> {
        self.client
            .add_note(
                self.id,
                &NoteData {
                    description: description.to_string(),
                    ..Default::default()
                },
            )
            .await
    }

    /// Start building a note with custom settings.
    pub fn note(self) -> NoteBuilder<'a> {
        NoteBuilder {
            client: self.client,
            ticket_id: self.id,
            description: String::new(),
            mark_first_response: false,
            add_to_linked_requests: false,
            notify_technician: false,
            show_to_requester: false,
        }
    }

    /// Merge other tickets into this one.
    pub async fn merge(self, ticket_ids: &[u64]) -> Result<(), Error> {
        let ids: Vec<usize> = ticket_ids.iter().map(|id| *id as usize).collect();
        self.client.merge(self.id.0 as usize, &ids).await
    }

    /// Edit ticket fields.
    pub async fn edit(self, data: &EditTicketData) -> Result<(), Error> {
        self.client.edit(self.id, data).await
    }

    /// Close ticket with a note.
    pub async fn close_with_note(self, comment: &str) -> Result<(), Error> {
        let id = self.id.clone();
        self.client
            .add_note(
                id.clone(),
                &NoteData {
                    description: comment.to_string(),
                    ..Default::default()
                },
            )
            .await?;
        self.client.close_ticket(id, comment).await
    }
}

/// Builder for searching tickets.
///
/// All filter methods are optional. Default limit is 100 results.
pub struct TicketSearchBuilder<'a> {
    client: &'a ServiceDesk,
    root_criteria: Option<Criteria>,
    children: Vec<Criteria>,
    row_count: u32,
}

/// Ticket status filter values.
#[derive(Debug)]
pub enum TicketStatus {
    Open,
    Closed,
    Cancelled,
    OnHold,
}

impl std::fmt::Display for TicketStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status_str = match self {
            TicketStatus::Open => "Open",
            TicketStatus::Closed => "Closed",
            TicketStatus::Cancelled => "Cancelled",
            TicketStatus::OnHold => "On Hold",
        };
        write!(f, "{}", status_str)
    }
}

impl<'a> TicketSearchBuilder<'a> {
    /// Filter by ticket status.
    pub fn status(mut self, status: &str) -> Self {
        self.root_criteria = Some(Criteria {
            field: "status.name".to_string(),
            condition: Condition::Is,
            value: status.into(),
            children: vec![],
            logical_operator: None,
        });
        self
    }

    /// Filter by ticket status using the [`TicketStatus`] enum.
    pub fn filter(self, filter: &TicketStatus) -> Self {
        self.status(&filter.to_string())
    }

    /// Filter by open tickets.
    pub fn open(self) -> Self {
        self.status("Open")
    }

    /// Filter by closed tickets.
    pub fn closed(self) -> Self {
        self.status("Closed")
    }

    /// Filter tickets created after a given time.
    pub fn created_after(mut self, time: DateTime<Local>) -> Self {
        self.children.push(Criteria {
            field: "created_time".to_string(),
            condition: Condition::GreaterThan,
            value: time.timestamp_millis().to_string().into(),
            children: vec![],
            logical_operator: Some(LogicalOp::And),
        });
        self
    }

    /// Filter tickets last updated after a given time.
    pub fn updated_after(mut self, time: DateTime<Local>) -> Self {
        self.children.push(Criteria {
            field: "last_updated_time".to_string(),
            condition: Condition::GreaterThan,
            value: time.timestamp_millis().to_string().into(),
            children: vec![],
            logical_operator: Some(LogicalOp::And),
        });
        self
    }

    /// Filter by subject containing a value.
    pub fn subject_contains(mut self, value: &str) -> Self {
        self.children.push(Criteria {
            field: "subject".to_string(),
            condition: Condition::Contains,
            value: value.into(),
            children: vec![],
            logical_operator: Some(LogicalOp::And),
        });
        self
    }

    /// Filter by a custom field containing a value.
    pub fn field_contains(mut self, field: &str, value: impl Into<Value>) -> Self {
        self.children.push(Criteria {
            field: field.to_string(),
            condition: Condition::Contains,
            value: value.into(),
            children: vec![],
            logical_operator: Some(LogicalOp::And),
        });
        self
    }

    /// Filter by a custom field matching exactly.
    pub fn field_equals(mut self, field: &str, value: impl Into<Value>) -> Self {
        self.children.push(Criteria {
            field: field.to_string(),
            condition: Condition::Is,
            value: value.into(),
            children: vec![],
            logical_operator: Some(LogicalOp::And),
        });
        self
    }

    /// Set maximum number of results. Default: 100.
    pub fn limit(mut self, count: u32) -> Self {
        self.row_count = count;
        self
    }

    /// Add a raw [`Criteria`] for complex queries.
    pub fn criteria(mut self, criteria: Criteria) -> Self {
        if self.root_criteria.is_none() {
            self.root_criteria = Some(criteria);
        } else {
            self.children.push(criteria);
        }
        self
    }

    /// Execute the search and return results.
    pub async fn fetch(self) -> Result<Vec<DetailedTicket>, Error> {
        let mut root = self.root_criteria.unwrap_or_else(|| Criteria {
            field: "id".to_string(),
            condition: Condition::GreaterThan,
            value: "0".into(),
            children: vec![],
            logical_operator: None,
        });

        root.children = self.children;

        let body = SearchRequest {
            list_info: ListInfo {
                row_count: self.row_count,
                search_criteria: root,
            },
        };

        let resp: Value = self
            .client
            .request_input_data(Method::GET, "/api/v3/requests", &body)
            .await?;

        let ticket_response: TicketSearchResponse = serde_json::from_value(resp)?;
        Ok(ticket_response.requests)
    }

    /// Execute the search and return the first result.
    pub async fn first(mut self) -> Result<Option<DetailedTicket>, Error> {
        self.row_count = 1;
        let results = self.fetch().await?;
        Ok(results.into_iter().next())
    }
}

/// Builder for creating tickets.
///
/// Required: [`subject`](Self::subject), [`requester`](Self::requester).
/// Default priority: "Low".
pub struct TicketCreateBuilder<'a> {
    client: &'a ServiceDesk,
    subject: Option<String>,
    description: Option<String>,
    requester: Option<String>,
    priority: String,
    account: Option<String>,
    template: Option<String>,
    udf_fields: Option<Value>,
}

impl<'a> TicketCreateBuilder<'a> {
    /// Set the ticket subject (required).
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    /// Set the ticket description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the requester name (required).
    pub fn requester(mut self, requester: impl Into<String>) -> Self {
        self.requester = Some(requester.into());
        self
    }

    /// Set the priority. Default: "Low".
    pub fn priority(mut self, priority: impl Into<String>) -> Self {
        self.priority = priority.into();
        self
    }

    /// Set the account name.
    pub fn account(mut self, account: impl Into<String>) -> Self {
        self.account = Some(account.into());
        self
    }

    /// Set the template name.
    pub fn template(mut self, template: impl Into<String>) -> Self {
        self.template = Some(template.into());
        self
    }

    /// Set custom UDF fields.
    pub fn udf_fields(mut self, fields: Value) -> Self {
        self.udf_fields = Some(fields);
        self
    }

    /// Create the ticket.
    pub async fn send(self) -> Result<TicketResponse, Error> {
        let subject = self
            .subject
            .ok_or_else(|| Error::Other("subject is required".to_string()))?;
        let requester = self
            .requester
            .ok_or_else(|| Error::Other("requester is required".to_string()))?;

        let data = CreateTicketData {
            subject,
            description: self.description.unwrap_or_default(),
            requester: NameWrapper::new(requester),
            priority: NameWrapper::new(self.priority),
            account: NameWrapper::new(self.account.unwrap_or_default()),
            template: NameWrapper::new(self.template.unwrap_or_default()),
            udf_fields: self.udf_fields.unwrap_or(serde_json::json!({})),
        };

        self.client.create_ticket(&data).await
    }
}

/// Builder for adding notes with custom settings.
///
/// All boolean options default to `false`.
pub struct NoteBuilder<'a> {
    client: &'a ServiceDesk,
    ticket_id: TicketID,
    description: String,
    mark_first_response: bool,
    add_to_linked_requests: bool,
    notify_technician: bool,
    show_to_requester: bool,
}

impl<'a> NoteBuilder<'a> {
    /// Set the note content.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Mark as first response.
    pub fn mark_first_response(mut self) -> Self {
        self.mark_first_response = true;
        self
    }

    /// Add to linked requests.
    pub fn add_to_linked_requests(mut self) -> Self {
        self.add_to_linked_requests = true;
        self
    }

    /// Notify the assigned technician.
    pub fn notify_technician(mut self) -> Self {
        self.notify_technician = true;
        self
    }

    /// Make visible to the requester.
    pub fn show_to_requester(mut self) -> Self {
        self.show_to_requester = true;
        self
    }

    /// Add the note.
    pub async fn send(self) -> Result<Note, Error> {
        let note = NoteData {
            description: self.description,
            mark_first_response: self.mark_first_response,
            add_to_linked_requests: self.add_to_linked_requests,
            notify_technician: self.notify_technician,
            show_to_requester: self.show_to_requester,
        };

        let note = self.client.add_note(self.ticket_id, &note).await?;
        Ok(note)
    }
}

impl ServiceDesk {
    /// Get a client for ticket collection operations.
    pub fn tickets(&self) -> TicketsClient<'_> {
        TicketsClient { client: self }
    }

    /// Get a client for single ticket operations.
    pub fn ticket(&self, id: impl Into<TicketID>) -> TicketClient<'_> {
        TicketClient {
            client: self,
            id: id.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticket_status_display() {
        assert_eq!(TicketStatus::Open.to_string(), "Open");
        assert_eq!(TicketStatus::Closed.to_string(), "Closed");
        assert_eq!(TicketStatus::Cancelled.to_string(), "Cancelled");
        assert_eq!(TicketStatus::OnHold.to_string(), "On Hold");
    }
}
