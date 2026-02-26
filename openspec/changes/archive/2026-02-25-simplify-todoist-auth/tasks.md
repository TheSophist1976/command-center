## 1. Dependencies (`Cargo.toml`)

- [x] 1.1 Remove `tiny_http` from `[dependencies]`
- [x] 1.2 Remove `open` from `[dependencies]`

## 2. CLI (`src/cli.rs`)

- [x] 2.1 Update `AuthCommand::Todoist` doc comment from "OAuth 2.0 browser flow" to "personal API token"
- [x] 2.2 Add `#[arg(long)] token: Option<String>` field to `AuthCommand::Todoist` for non-interactive use

## 3. Auth Module (`src/auth.rs`)

- [x] 3.1 Remove `run_oauth_flow`, `listen_for_callback`, `extract_code_from_url`, `exchange_code`, and the `_use_read` shim function
- [x] 3.2 Remove the `use std::io::Read` import (no longer needed)
- [x] 3.3 Add `pub fn prompt_for_token(token_flag: Option<String>) -> Result<String, String>`: if `token_flag` is `Some`, use it; otherwise print the Todoist settings URL, prompt "Paste your Todoist API token: ", read a line from stdin, trim it, and return an error if empty

## 4. Main Dispatch (`src/main.rs`)

- [x] 4.1 Update `AuthCommand::Todoist` handler: remove `TODOIST_CLIENT_ID`/`TODOIST_CLIENT_SECRET` env var reads and `run_oauth_flow` call; replace with `auth::prompt_for_token(token)` call, passing the `token` field from the CLI variant
- [x] 4.2 Update the `AuthCommand::Todoist` match arm to destructure `{ token }` from the variant

## 5. Verification

- [x] 5.1 Run `cargo build` with zero errors and zero warnings
- [x] 5.2 Smoke test interactive path: `task auth todoist` prompts for a token and stores it; `task auth status` reports "present"
- [x] 5.3 Smoke test flag path: `task auth todoist --token abc123` stores the token without prompting
- [x] 5.4 Smoke test revoke: `task auth revoke` removes the token; `task auth status` reports "not set"
