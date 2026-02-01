# sdp-request-client

Async Rust client for the ManageEngine ServiceDesk Plus REST API v3.

## Installation

```toml
[dependencies]
sdp-request-client = "0.1"
```

## Usage

```rust
use sdp_request_client::{ServiceDesk, ServiceDeskOptions, Credentials};
use reqwest::Url;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ServiceDesk::new(
        Url::parse("https://sdp.example.com")?,
        Credentials::Token { token: "YOUR_TOKEN".into() },
        ServiceDeskOptions::default(),
    );

    // Search for open tickets
    let tickets = client.tickets()
        .search()
        .open()
        .limit(10)
        .fetch()
        .await?;

    // Create a ticket
    let response = client.tickets()
        .create()
        .subject("Server Down")
        .requester("John Doe")
        .priority("High")
        .send()
        .await?;

    // Add a note
    client.ticket(12345).add_note("Investigating...").await?;

    // Close a ticket
    client.ticket(12345).close("Resolved").await?;

    Ok(())
}
```

## License

MIT
