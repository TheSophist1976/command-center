## Why

The current `task auth todoist` command implements a full OAuth 2.0 browser flow — opening a browser, spinning up a local HTTP server on port 7777, capturing an authorization code, and exchanging it for an access token. This requires `TODOIST_CLIENT_ID` and `TODOIST_CLIENT_SECRET` env vars and a registered OAuth app, which is unnecessary friction for a personal CLI tool.

Todoist provides a personal API token in account settings that works directly with the REST API. Replacing the OAuth flow with a simple token-paste UX removes the app registration requirement, eliminates `tiny_http` and `open` dependencies, and makes setup a single step.

## What Changes

- Replace `task auth todoist` implementation: instead of launching a browser OAuth flow, prompt the user to paste their personal API token from Todoist settings (`https://app.todoist.com/app/settings/integrations/developer`)
- Remove `run_oauth_flow`, `listen_for_callback`, `extract_code_from_url`, and `exchange_code` from `src/auth.rs`
- Remove `tiny_http` and `open` crates from `Cargo.toml` (no longer needed)
- `TODOIST_CLIENT_ID` and `TODOIST_CLIENT_SECRET` env vars are no longer required
- Token storage path, `read_token`, `write_token`, `delete_token`, `auth status`, and `auth revoke` are unchanged

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `todoist-auth`: Replace OAuth 2.0 browser flow with a personal API token prompt. `task auth todoist` prints a URL to Todoist settings and reads the token from stdin (or `--token <value>` flag for non-interactive use). All other behavior (token storage, revoke, status) is unchanged.

## Impact

- **Removed dependencies**: `tiny_http`, `open`
- **Code**: `src/auth.rs` (replace OAuth functions with stdin prompt), `src/main.rs` (remove env var reads for client_id/client_secret), `Cargo.toml` (remove deps)
- **No storage changes**: token file path and format unchanged
- **Simpler UX**: `task auth todoist` → prints instructions + URL → user pastes token → done
