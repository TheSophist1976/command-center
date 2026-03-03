### Requirement: URL fetch tool
The NLP module SHALL provide a `fetch_url` tool that the Claude model can invoke via tool_use. The tool SHALL accept a `url` string parameter, fetch the page content via HTTP GET, extract readable text from the HTML, and return the extracted text (truncated to 4000 characters) as a tool_result.

#### Scenario: Model requests URL fetch
- **WHEN** the model responds with a `tool_use` block containing `name: "fetch_url"` and `input: {"url": "https://example.com/article"}`
- **THEN** the system SHALL fetch the URL, extract text content from the HTML, and return it as a tool_result

#### Scenario: Text extraction from HTML
- **WHEN** a URL returns HTML content
- **THEN** the system SHALL extract text from semantic elements (paragraphs, headings, list items) and strip HTML tags, scripts, and styles

#### Scenario: Content truncation
- **WHEN** the extracted text exceeds 4000 characters
- **THEN** the system SHALL truncate to 4000 characters and append a note indicating truncation

#### Scenario: Fetch error
- **WHEN** the URL is unreachable, returns a non-200 status, or times out
- **THEN** the system SHALL return a tool_result with an error message describing the failure

#### Scenario: Request timeout
- **WHEN** the URL fetch takes longer than 10 seconds
- **THEN** the system SHALL abort the request and return a timeout error as the tool_result

### Requirement: Tool-use loop
The `call_claude_api` function SHALL support a multi-step tool-use loop. After sending the initial request, if the response contains `tool_use` blocks, the system SHALL execute the requested tools, append the results as `tool_result` messages, and re-send to the API. This loop SHALL repeat until the model returns a final text response or the iteration limit is reached.

#### Scenario: Single tool call
- **WHEN** the model's first response contains one `tool_use` block
- **THEN** the system SHALL execute the tool, send the result back, and return the model's final text response

#### Scenario: Iteration limit
- **WHEN** the tool-use loop reaches 3 iterations without a final text response
- **THEN** the system SHALL stop and return an error message to the user

#### Scenario: No tools requested
- **WHEN** the model's response contains only text (no tool_use blocks)
- **THEN** the system SHALL return the text directly without entering the tool-use loop

### Requirement: Tool definitions in API request
The API request SHALL include a tool definition for `fetch_url` in the `tools` array. The tool definition SHALL describe the tool's purpose and input schema so the model knows when and how to use it.

#### Scenario: Tool definition included
- **WHEN** an NLP request is made to the Claude API
- **THEN** the request SHALL include a `tools` array with a definition for `fetch_url` including its name, description, and input_schema
