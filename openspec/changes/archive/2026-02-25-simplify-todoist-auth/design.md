## Context

The existing `task auth todoist` command implements a full OAuth 2.0 browser flow in `src/auth.rs`: it builds an authorization URL, opens the user's browser via the `open` crate, spins up a `tiny_http` server on `127.0.0.1:7777` to capture the redirect, extracts the `?code=` query param, and exchanges it for an access token via a POST to Todoist's token endpoint. This requires the caller to have registered a Todoist OAuth app and set `TODOIST_CLIENT_ID` and `TODOIST_CLIENT_SECRET` env vars.

Todoist accounts come with a personal API token, visible at `https://app.todoist.com/app/settings/integrations/developer`. This token has the same data access as an OAuth token for personal use and requires no app registration.

## Goals / Non-Goals

**Goals:**
- Replace the OAuth flow in `src/auth.rs` with a simple stdin prompt that accepts the user's personal API token
- Support a `--token <value>` flag on `task auth todoist` for non-interactive/scripted use
- Remove the `tiny_http` and `open` crate dependencies from `Cargo.toml`
- Remove the `TODOIST_CLIENT_ID` and `TODOIST_CLIENT_SECRET` env var reads from `src/main.rs`
- Keep token storage path, format, `read_token`, `write_token`, `delete_token`, `auth status`, and `auth revoke` exactly as-is

**Non-Goals:**
- Supporting multi-account or workspace token management
- Validating the token against the API before storing it
- Changing the token file location or format

## Decisions

**1. Stdin prompt, not raw `read_line`**
Print a clear instruction message with the Todoist settings URL, then read a single line from stdin. Trim whitespace. If the result is empty, return an error. This keeps the UX simple and works in all terminals.

**2. `--token` flag for non-interactive use**
Add `#[arg(long)] token: Option<String>` to the `Auth::Todoist` CLI variant. If provided, skip the prompt and use the value directly. This allows scripts and CI to set the token without stdin interaction.

**3. Remove `run_oauth_flow`, `listen_for_callback`, `extract_code_from_url`, `exchange_code`, and the unused `_use_read` shim from `src/auth.rs`**
These functions are entirely replaced. The `Read` import and `tiny_http` usage go away with them.

**4. Remove `tiny_http` and `open` from `Cargo.toml`**
These crates were only used for the OAuth callback server and browser launch. With a token-paste flow, neither is needed.

**5. No API validation on store**
The token is stored as-is without hitting the API. If the token is wrong, the user will find out on the next `task import todoist` call, which already handles the 401 case with a clear error message.

## Risks / Trade-offs

- **User error**: Pasting an incorrect token produces no immediate feedback. Mitigated by the existing 401 error in `todoist.rs::fetch_open_tasks`.
- **Security**: Token appears in terminal history if passed via `--token`. Acceptable for a personal CLI; the prompt path avoids this.
