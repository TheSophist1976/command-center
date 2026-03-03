## Why

The AI chat in the TUI can answer questions about tasks using the local task data, but it cannot access external content. Users often include URLs in task titles or descriptions, and may want the AI to summarize linked content. Adding URL fetching gives the AI the ability to read and summarize web pages, making it more useful as a task assistant.

## What Changes

- Add a `fetch_url` tool that the Claude model can invoke via tool_use to retrieve and summarize content from a URL
- Implement URL fetching with HTML-to-text extraction for summarization
- Update the system prompt to inform the model about the fetch_url tool and when to use it
- Add a tool-use loop: model requests fetch → app executes it → results sent back to model → model responds to user
- No new API keys required — uses existing `reqwest` for HTTP fetching

## Capabilities

### New Capabilities
- `nlp-url-fetch`: URL fetching capability for the NLP assistant, including the fetch_url tool, HTML-to-text extraction, and tool-use response flow

### Modified Capabilities
- `nlp-conversation`: Update system prompt to describe the fetch_url tool; extend the API call flow to support tool-use patterns where the model can request external data

## Impact

- `src/nlp.rs`: Tool-use loop in API call, fetch_url execution, HTML-to-text extraction, updated system prompt, tool definitions in API request
- `src/tui.rs`: Show loading state during URL fetch requests
- `Cargo.toml`: May need `scraper` or similar crate for HTML parsing (already has `reqwest`)
