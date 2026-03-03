## Context

The NLP chat currently calls the Claude API with a system prompt containing task data, and the model responds with a single JSON action (filter, update, message, show_tasks, set_recurrence). All intelligence is confined to the local task data — the model cannot access external content. Users often include URLs in tasks and want the AI to summarize linked pages.

The app already uses `reqwest` (blocking) for the Claude API. The Claude API supports a tool-use pattern where the model can request tool calls, the client executes them, and results are sent back in a follow-up request.

## Goals / Non-Goals

**Goals:**
- Let the model fetch and summarize URLs found in task titles/descriptions or provided by the user
- Keep the existing action-based response format working alongside the new capability
- No new API keys or external services required

**Non-Goals:**
- No web search — just URL fetching (web search can be added later)
- No persistent caching of fetched pages
- No browser rendering or JavaScript execution — plain HTTP fetch with HTML-to-text
- Not replacing the existing NLP action system — URL fetch augments it

## Decisions

### 1. Use Claude tool_use for URL fetching

Rather than adding a new NLP action type that the TUI must handle, use Claude's native tool_use feature. The model decides when it needs to read a URL, emits a `tool_use` block, the app fetches the page, sends content back as `tool_result`, and the model incorporates the data into its final JSON action response.

**Why not a new NlpAction variant?** A tool-use loop keeps the complexity in `nlp.rs` — the TUI doesn't need to know about URL fetching at all. The model's final response is still the same JSON action format (message, filter, etc.).

### 2. One tool: `fetch_url`

`fetch_url(url: string)` — fetches a URL and returns extracted text content (truncated to fit context).

The model uses this when the user asks about a link, says "summarize this URL", or references a task that contains a URL.

### 3. Simple HTML-to-text extraction

Use `reqwest` to fetch the URL, then strip HTML tags with a lightweight approach — either a small helper function or the `scraper` crate to extract text from `<p>`, `<h1>`-`<h6>`, `<li>`, `<td>` elements. Strip `<script>` and `<style>` content. Truncate to ~4000 chars to keep context manageable.

**Alternative considered:** Adding a full HTML parser or headless browser. Overkill for summarization — we just need readable text.

### 4. Tool-use loop with max iterations

The `call_claude_api` function gains a loop: send request → check if response contains `tool_use` blocks → execute tool → send results back. Cap at 3 iterations to prevent runaway loops. If the model doesn't converge to a final text response, return an error message.

### 5. Tool definitions conditional on nothing (always available)

Since `fetch_url` requires no API key (just HTTP), the tool is always available. The tool definition is always included in the API request.

## Risks / Trade-offs

- [Slow responses] URL fetch adds 1-3 seconds of latency → Show a "Fetching..." status message in the TUI while waiting.
- [Content quality] HTML-to-text extraction may produce noisy output → Truncate aggressively and let Claude summarize the relevant parts.
- [API cost] Tool-use requires an extra API round-trip → Using haiku keeps cost minimal. Cap iterations at 3.
- [Blocked URLs] Some sites block non-browser requests → Set a reasonable User-Agent header. Accept that some URLs won't work.
- [Large pages] Very large HTML pages could be slow to download → Set a timeout (10 seconds) and max response body size.
