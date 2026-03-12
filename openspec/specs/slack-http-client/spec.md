## ADDED Requirements

### Requirement: Shared HTTP client for Slack API calls
All Slack API functions SHALL accept a shared `reqwest::blocking::Client` reference instead of creating a new client per request. The shared client SHALL be created once per sync operation and passed to all API call sites.

#### Scenario: Client reuse across API calls
- **WHEN** a Slack sync operation begins
- **THEN** a single `reqwest::blocking::Client` SHALL be created and passed to all subsequent API calls (fetch_channels, fetch_channel_info, fetch_messages, fetch_user_info, mark_read, send_message, fetch_workspace_name)

#### Scenario: Connection pooling within a sync
- **WHEN** multiple API calls are made to `slack.com` during a single sync
- **THEN** the shared client SHALL reuse TCP/TLS connections via its internal connection pool

### Requirement: Concurrent per-channel fetch
The system SHALL fetch channel info and messages for multiple channels concurrently during inbox sync, using scoped threads with a bounded concurrency limit.

#### Scenario: Parallel channel processing
- **WHEN** the inbox sync processes 15 configured channels
- **THEN** the system SHALL fetch channel info and messages for up to 6 channels concurrently using `std::thread::scope`

#### Scenario: Single channel falls back to sequential
- **WHEN** the inbox sync processes 1 configured channel
- **THEN** the system SHALL process it on a single thread without spawning additional threads

#### Scenario: Per-channel error isolation
- **WHEN** a channel fetch fails (API error, rate limit, network error) during concurrent processing
- **THEN** the error SHALL be logged for that channel and other channel fetches SHALL continue unaffected

### Requirement: Concurrent conversation type fetches
The `fetch_channels` function SHALL fetch all conversation types (public_channel, private_channel, mpim, im) concurrently instead of sequentially.

#### Scenario: Parallel type fetches
- **WHEN** `fetch_channels` is called with types "public_channel,private_channel,mpim,im"
- **THEN** the system SHALL initiate fetches for all 4 types concurrently using scoped threads

#### Scenario: Missing scope for one type does not block others
- **WHEN** the token lacks scope for `mpim` conversations but has scope for the other 3 types
- **THEN** the system SHALL return channels from the 3 successful types and include a scope warning for `mpim`

### Requirement: Batched concurrent user resolution
The system SHALL collect all unknown user IDs across all channels after message fetching, deduplicate them, and resolve them concurrently in a single batch.

#### Scenario: Cross-channel deduplication
- **WHEN** messages from 3 different channels reference the same user ID `U12345`
- **THEN** the system SHALL resolve `U12345` exactly once, not three times

#### Scenario: Parallel user info fetches
- **WHEN** 8 unknown user IDs need resolution after message fetching
- **THEN** the system SHALL resolve them concurrently (up to 6 at a time) using scoped threads

#### Scenario: Cached users are not re-fetched
- **WHEN** a user ID is already present in the user cache
- **THEN** the system SHALL NOT make an API call for that user, even during batch resolution

### Requirement: Concurrency limit
All concurrent operations SHALL be bounded to a maximum of 6 simultaneous threads to stay within Slack API rate limits.

#### Scenario: More channels than thread cap
- **WHEN** 15 channels need fetching with a concurrency limit of 6
- **THEN** the system SHALL process channels in batches of 6, waiting for each batch to complete before starting the next

#### Scenario: Fewer channels than thread cap
- **WHEN** 3 channels need fetching with a concurrency limit of 6
- **THEN** the system SHALL spawn 3 threads (not 6) and process all channels in a single batch
