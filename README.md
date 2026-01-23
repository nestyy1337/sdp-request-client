# sdp-request-client

An asynchronous Rust client for the ManageEngine ServiceDesk Plus (SDP) REST API v3.
### PoC client for SDP's [Request](https://www.manageengine.com/products/service-desk/sdpod-v3-api/requests/request.html) API

Possible that it won't ever be updated but I could extend the functionality to other components of SDP's API if needed.
Currently not optimized for minimal allocations, I will take a look into that as soon as the public API matures.

Tests are connecting directly to SDP instance so to test your own setup you should fork the repo and adjust the tests.



### Initialize the Client

```rust
use sdp_request_client::{ServiceDesk, ServiceDeskOptions, Credentials};
use reqwest::Url;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let credentials = Credentials::Token {
        token: "YOUR_SDP_TOKEN".to_string(),
    };

    let sdp = ServiceDesk::new(
        Url::parse("https://sdp.example.com")?,
        credentials,
        ServiceDeskOptions::default(),
    );

    Ok(())
}
```

### Search for Tickets

```rust
let tickets = sdp.tickets()
    .search()
    .open()
    .subject_contains("[ALERT]")
    .limit(10)
    .fetch()
    .await?;

for ticket in tickets {
    println!("{}: {}", ticket.id, ticket.subject);
}
```

### Create a Ticket

```rust
let response = sdp.tickets()
    .create()
    .subject("Server Down")
    .description("The main production server is unresponsive.")
    .requester("John Doe")
    .priority("High")
    .account("Internal IT")
    .send()
    .await?;

println!("Created ticket: {}", response.request.id);
```

### Ticket Operations (Notes, Assignment, Closing)

```rust
let ticket_id = 12345;

// Add a simple note
sdp.ticket(ticket_id).add_note("Investigating the issue...").await?;

// Add a note with specific options
sdp.ticket(ticket_id)
    .note()
    .description("Visible to requester")
    .show_to_requester()
    .send()
    .await?;

// Assign to a technician
sdp.ticket(ticket_id).assign("Jane Smith").await?;

// Close the ticket
sdp.ticket(ticket_id).close("Issue resolved by restart.").await?;
```

