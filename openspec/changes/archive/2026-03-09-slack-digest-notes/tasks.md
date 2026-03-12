## 1. Configuration

- [ ] 1.1 Add `slack-workspace` config key support — read/write via existing `config::read_config_value` / `config::write_config_value` and expose in CLI `task config set slack-workspace <domain>`
- [ ] 1.2 Add integration test verifying `slack-workspace` round-trips through config read/write

## 2. Permalink Construction

- [ ] 2.1 Add `build_permalink(workspace: &str, channel_id: &str, ts: &str) -> String` function in `src/slack.rs` that strips the dot from the timestamp and returns the full permalink URL
- [ ] 2.2 Add unit tests for `build_permalink` — normal case, missing dot in ts, various timestamp formats

## 3. Digest Analysis

- [ ] 3.1 Define `SlackDigestResult` struct in `src/slack.rs` with `summary: String` and `suggestions: Vec<SlackInboxMessage>` (or appropriate action item type)
- [ ] 3.2 Implement `analyze_slack_digest` function that builds a Claude prompt requesting both a markdown summary and action items, sends it, and parses the JSON response into `SlackDigestResult`
- [ ] 3.3 In the prompt, include pre-built permalinks alongside each message (channel ID, sender, timestamp, text, permalink) so the AI can embed them as markdown links
- [ ] 3.4 Handle the case where workspace is not configured — pass messages without permalinks and instruct the AI to omit links

## 4. Digest Note Creation

- [ ] 4.1 Add a `write_note_with_slug` helper in `src/note.rs` (or extend `write_note`) that accepts an explicit slug instead of deriving from title, using `unique_slug` for collision avoidance
- [ ] 4.2 Implement digest note creation logic: generate slug `slack-digest-YYYY-MM-DD`, set title `Slack Digest YYYY-MM-DD`, write the AI summary as the note body
- [ ] 4.3 Add unit test for slug override and collision avoidance (e.g., second digest same day gets `-2` suffix)

## 5. TUI Flow Integration

- [ ] 5.1 Add a new TUI flow entry point (or extend the existing Slack inbox review) that triggers `analyze_slack_digest` with the current inbox messages
- [ ] 5.2 After digest analysis, save the digest note immediately and display a status message confirming the note slug (e.g., "Saved digest note: slack-digest-2026-03-05")
- [ ] 5.3 Transition to existing Slack inbox review mode with the extracted action items so the user can accept/skip/edit tasks as before
- [ ] 5.4 Handle edge case: no new messages — show status message and skip digest generation

## 6. Testing & Polish

- [ ] 6.1 Add integration test for the full digest flow: mock messages → analyze → note created → action items returned
- [ ] 6.2 Add graceful degradation test: no workspace configured → digest created without permalinks, hint shown
- [ ] 6.3 Verify existing Slack review flow still works unchanged (backward compatibility)
