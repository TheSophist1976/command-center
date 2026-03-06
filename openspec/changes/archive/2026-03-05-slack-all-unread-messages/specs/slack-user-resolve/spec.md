## ADDED Requirements

### Requirement: Resolve Slack user IDs to display names
The system SHALL resolve Slack user IDs to human-readable display names using the Slack `users.info` API.

#### Scenario: Resolve a single user ID
- **WHEN** the system needs to display a user's name (e.g., for a DM label)
- **THEN** it SHALL call the Slack `users.info` API with the user ID and extract the display name (preferring `profile.display_name`, falling back to `profile.real_name`, then `name`)

#### Scenario: User ID resolution fails
- **WHEN** the `users.info` API call fails (network error, rate limit, or invalid user ID)
- **THEN** the system SHALL fall back to displaying the raw user ID (e.g., `U12345`) without blocking the review flow

### Requirement: File-backed user name cache
The system SHALL cache resolved user ID → display name mappings in a JSON file to avoid redundant API calls across sessions.

#### Scenario: Cache file location
- **WHEN** the system reads or writes the user cache
- **THEN** it SHALL use the path `<config_dir>/task-manager/slack_users.json` (same directory as `slack_state.json`)

#### Scenario: Cache hit — no API call
- **WHEN** a user ID is found in the cache
- **THEN** the system SHALL return the cached display name without making an API call

#### Scenario: Cache miss — fetch and store
- **WHEN** a user ID is NOT found in the cache
- **THEN** the system SHALL call `users.info`, store the result in the cache, and persist the cache to disk

#### Scenario: Cache file does not exist
- **WHEN** the cache file does not exist on disk
- **THEN** the system SHALL treat it as an empty cache and create the file on first write

### Requirement: Batch resolution for conversation display names
The system SHALL resolve all unique user IDs needed for conversation display names in a batch before entering the review UI, to minimize per-message lookups.

#### Scenario: Resolve users for IM conversations
- **WHEN** the fetched conversations include IM conversations
- **THEN** the system SHALL collect all unique `user` IDs from IM conversations and resolve them before constructing display names

#### Scenario: Resolve users for MPIM conversations
- **WHEN** the fetched conversations include MPIM (group DM) conversations
- **THEN** the system SHALL resolve participant user IDs needed for the group display name

#### Scenario: Resolve users for message authors
- **WHEN** messages are fetched and contain `user` fields with IDs
- **THEN** the system SHALL resolve those user IDs so the NLP analysis prompt includes human-readable author names instead of raw IDs
