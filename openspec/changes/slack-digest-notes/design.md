## Context

The Slack review flow (`S` key in TUI) currently fetches new messages across configured channels, sends them to Claude for action item extraction, and presents each suggestion one-by-one for accept/skip/edit. The result is a set of tasks — but the raw conversation context is lost.

The existing infrastructure includes:
- `slack.rs`: `fetch_new_messages`, `analyze_slack_messages` (Claude prompt → `Vec<SlackSuggestion>`)
- `note.rs`: `Note` struct (slug, title, body), `write_note`, `slugify`, `unique_slug`
- `tui.rs`: `start_slack_review` orchestrates fetch → resolve users → analyze → enter `SlackReview` mode

Permalink format: `https://<workspace>.slack.com/archives/<channel_id>/p<ts_without_dot>` (e.g., ts `1709654321.000100` → `p1709654321000100`).

## Goals / Non-Goals

**Goals:**
- Generate a markdown digest note summarizing Slack messages with permalinks, organized by channel
- Extract action items as tasks (preserving existing behavior)
- Combine summary + action items in a single Claude API call to avoid double cost
- Let the user preview and confirm the digest note before saving
- Auto-generate a date-stamped note slug (e.g., `slack-digest-2026-03-05`)

**Non-Goals:**
- Replacing the existing action-item-only review flow (this extends it)
- Fetching workspace domain automatically via Slack API (user configures it via `task config set slack-workspace <domain>`)
- Thread-level summarization (messages are treated as flat channel history)
- Editing the digest note inline in the TUI before saving (user can edit it later in the note editor)

## Decisions

### 1. Single Claude API call for both summary and action items

**Choice:** Extend the `analyze_slack_messages` prompt to return a JSON object with two fields: `summary` (markdown string organized by channel) and `action_items` (existing array format).

**Alternatives considered:**
- Two separate API calls (one for summary, one for tasks): simpler parsing but doubles cost and latency
- Summary only, derive tasks separately in a follow-up step: loses the atomic context

**Rationale:** A single call keeps cost and latency constant. The prompt already has all messages in context; asking for both outputs is straightforward. The response format changes from a JSON array to a JSON object wrapping both.

### 2. Permalink construction in Rust, not in the AI prompt

**Choice:** Build permalinks in `slack.rs` from channel ID + message timestamp. Pass them pre-built into the prompt context so the AI can reference them in the summary.

**Alternatives considered:**
- Ask the AI to generate permalinks: unreliable, would need workspace domain in the prompt
- Generate permalinks post-hoc and inject into the AI summary: fragile text manipulation

**Rationale:** Permalink format is deterministic (`/archives/{channel}/p{ts_no_dot}`). Building them in Rust is reliable and lets the AI reference them naturally as `[message](url)` in its summary output.

### 3. Workspace domain from config

**Choice:** Store the Slack workspace domain (e.g., `myteam.slack.com`) via `task config set slack-workspace myteam`. Required for permalink generation.

**Alternatives considered:**
- Fetch via `auth.test` or `team.info` API: adds an API call and requires additional scopes
- Omit permalinks entirely: loses a key value prop of the digest

**Rationale:** The workspace domain rarely changes and is easy for users to provide. Avoids extra API calls and scope requirements. If not configured, permalinks are omitted and a warning is shown.

### 4. Digest note as a standard Note with naming convention

**Choice:** Create digest notes using the existing `Note` struct and `write_note` function. The slug follows the pattern `slack-digest-YYYY-MM-DD` (with `unique_slug` handling conflicts for multiple digests per day). No metadata field changes to `Note`.

**Alternatives considered:**
- Add a `source` or `metadata` field to `Note`: more structured but requires schema migration, affects all note operations
- Separate storage for digests: duplicates infrastructure

**Rationale:** The naming convention is sufficient to identify digest notes. The existing note infrastructure handles everything needed (create, read, display, delete). Adding metadata can be done later if filtering/grouping becomes important. This keeps the change minimal.

### 5. Flow integration: digest note created alongside task review

**Choice:** After the AI returns both summary and action items, the digest note is saved automatically and the user enters the existing `SlackReview` mode for task review. A status message confirms the note was saved with its slug.

**Alternatives considered:**
- Show a full preview of the digest note before saving: adds a new TUI mode/screen, complexity
- Ask user to confirm save vs. discard: extra interaction for something that's almost always wanted
- Create the note only after all tasks are reviewed: delays the note, risk of losing it if user exits early

**Rationale:** The digest note is a record of what was discussed — it's always useful. Saving it immediately and showing a confirmation message keeps the flow fast. The user can always delete or edit the note later. The task review flow remains unchanged.

## Risks / Trade-offs

- **[Larger AI response]** Asking for summary + action items increases response size. → Mitigation: The summary is bounded by the input message count. Set a reasonable `max_tokens` (8192 should suffice).
- **[Permalink requires config]** Users must set `slack-workspace` for links to work. → Mitigation: Degrade gracefully — omit links if not configured, show a one-time hint.
- **[Response format change]** The `analyze_slack_messages` return type changes from `Vec<SlackSuggestion>` to a new struct containing both summary and suggestions. → Mitigation: Create a new function `analyze_slack_digest` rather than modifying the existing one, preserving backward compatibility.
- **[Note clutter]** Frequent Slack reviews could create many digest notes. → Mitigation: Clear naming convention makes them easy to identify and bulk-delete. Future enhancement could auto-archive old digests.
