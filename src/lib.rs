//! Async Rust client for the ManageEngine ServiceDesk Plus REST API v3.
//!
//! # Quick Start
//!
//! ```no_run
//! use sdp_request_client::{ServiceDesk, ServiceDeskOptions, Credentials};
//! use reqwest::Url;
//!
//! # async fn example() -> Result<(), sdp_request_client::Error> {
//! let client = ServiceDesk::new(
//!     Url::parse("https://sdp.example.com")?,
//!     Credentials::Token { token: "your-token".into() },
//!     ServiceDeskOptions::default(),
//! )?;
//!
//! // Search tickets
//! let tickets = client.tickets().search().open().limit(10).fetch().await?;
//!
//! // Create a ticket
//! let response = client.tickets()
//!     .create()
//!     .subject("Server issue")
//!     .requester("John Doe")
//!     .send()
//!     .await?;
//!
//! // Ticket operations
//! client.ticket(12345).add_note("Investigating...").await?;
//! client.ticket(12345).close("Resolved").await?;
//! # Ok(())
//! # }
//! ```
//!
//! See [`ServiceDesk`] for the main entry point.

use std::time::Duration;

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
    Account, Attachment, Condition, CreateTicketData, Criteria, DetailedTicket, EditTicketData,
    LogicalOp, Note, NoteData, Priority, Resolution, Status, TemplateInfo, TicketData, TimeEntry,
    UserInfo,
};
pub use error::Error;

/// Type-safe wrapper for User ID in SDP
#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq, Hash, Default)]
pub struct UserID(pub String);

/// Type-safe wrapper for Ticket ID in SDP
///
/// Deserializes from both numbers (`123`) and strings (`"123"`),
/// since the SDP API returns IDs as strings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct TicketID(pub u64);

/// Type-safe wrapper for Note ID in SDP
///
/// Deserializes from both numbers (`123`) and strings (`"123"`),
/// since the SDP API returns IDs as strings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct NoteID(pub u64);

/// Visitor that accepts either a number or a string and parses to u64.
struct StringOrNumberU64Visitor;

impl<'de> serde::de::Visitor<'de> for StringOrNumberU64Visitor {
    type Value = u64;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a u64 or a string containing a u64")
    }

    fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<u64, E> {
        Ok(v)
    }

    fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<u64, E> {
        u64::try_from(v).map_err(|_| E::custom(format!("negative id: {v}")))
    }

    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<u64, E> {
        v.parse::<u64>().map_err(serde::de::Error::custom)
    }
}

impl<'de> Deserialize<'de> for TicketID {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer
            .deserialize_any(StringOrNumberU64Visitor)
            .map(TicketID)
    }
}

impl<'de> Deserialize<'de> for NoteID {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer
            .deserialize_any(StringOrNumberU64Visitor)
            .map(NoteID)
    }
}

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

/// Main client for interacting with ServiceDesk Plus API.
///
/// Use [`tickets()`](Self::tickets) for search/create operations,
/// or [`ticket(id)`](Self::ticket) for single-ticket operations.
#[derive(Clone)]
pub struct ServiceDesk {
    base_url: Url,
    inner: reqwest::Client,
}

/// Security options for the ServiceDesk client
///
/// Not finished yet!!
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Security {
    Unsafe,
    NativeTLS,
}

/// Configuration options for the ServiceDesk client
#[derive(Clone, Debug)]
pub struct ServiceDeskOptions {
    pub user_agent: Option<String>,
    /// Request timeout duration
    pub timeout: Option<Duration>,
    pub security: Option<Security>,
    pub default_headers: Option<HeaderMap>,
}

static SDP_HEADER: (HeaderName, HeaderValue) = (
    HeaderName::from_static("accept"),
    HeaderValue::from_static("application/vnd.manageengine.sdp.v3+json"),
);

impl Default for ServiceDeskOptions {
    fn default() -> Self {
        ServiceDeskOptions {
            user_agent: Some(String::from("servicedesk-rs/0.1.0")),
            timeout: Some(Duration::from_secs(5)),
            security: Some(Security::Unsafe),
            default_headers: Some(HeaderMap::from_iter(vec![SDP_HEADER.clone()])),
        }
    }
}

impl ServiceDesk {
    /// Create a new ServiceDesk client instance.
    ///
    /// # Errors
    ///
    /// Returns an error if the auth token contains invalid header characters
    /// or if the underlying HTTP client fails to build.
    pub fn new(
        base_url: Url,
        credentials: Credentials,
        options: ServiceDeskOptions,
    ) -> Result<Self, Error> {
        let mut headers = options.default_headers.unwrap_or_default();

        if let Credentials::Token { ref token } = credentials {
            let value = HeaderValue::from_str(token)
                .map_err(|e| Error::Other(format!("invalid auth token header value: {e}")))?;
            headers.insert("authtoken", value);
        }

        let mut builder = reqwest::ClientBuilder::new()
            .default_headers(headers)
            .user_agent(options.user_agent.unwrap_or_default())
            .timeout(options.timeout.unwrap_or_else(|| Duration::from_secs(5)));

        if let Some(security) = options.security {
            match security {
                Security::Unsafe => {
                    builder = builder.danger_accept_invalid_certs(true);
                }
                Security::NativeTLS => {}
            }
        }

        let inner = builder
            .build()
            .map_err(|e| Error::Other(format!("failed to build HTTP client: {e}")))?;

        Ok(ServiceDesk { base_url, inner })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_desk_options_default() {
        let opts = ServiceDeskOptions::default();
        assert_eq!(opts.user_agent, Some("servicedesk-rs/0.1.0".to_string()));
        assert_eq!(opts.timeout, Some(Duration::from_secs(5)));
        assert!(matches!(opts.security, Some(Security::Unsafe)));
        assert!(opts.default_headers.is_some());
    }
}
