### Requirement: Markdown line styling
The system SHALL provide a function that accepts a line of markdown text and a code-block context flag, and returns a `Vec<Span>` with appropriate ratatui styles applied. The function SHALL also return the updated code-block context flag.

#### Scenario: Plain text line
- **WHEN** the input line is "Hello world" with no markdown syntax
- **THEN** the output SHALL be a single unstyled Span containing "Hello world"

### Requirement: Heading rendering
The system SHALL detect lines starting with `# `, `## `, or `### ` (with space after hashes) and style the entire line as a heading. `#` headings SHALL be bold with bright color, `##` SHALL be bold, and `###` SHALL be bold with dimmed color.

#### Scenario: H1 heading
- **WHEN** the input line is "# My Title"
- **THEN** the output SHALL be a single Span with bold and bright-colored style containing "# My Title"

#### Scenario: H2 heading
- **WHEN** the input line is "## Section"
- **THEN** the output SHALL be a single Span with bold style containing "## Section"

#### Scenario: Hash without space is not a heading
- **WHEN** the input line is "#notaheading"
- **THEN** the line SHALL NOT be styled as a heading

### Requirement: Code block rendering
The system SHALL track fenced code block state across lines. Lines consisting of `` ``` `` (optionally followed by a language identifier) SHALL toggle code block mode. Lines inside a code block SHALL be styled with a distinct color (e.g., Green) and SHALL NOT have inline markdown parsing applied.

#### Scenario: Enter and exit code block
- **WHEN** the lines are ["```", "let x = 1;", "```"]
- **THEN** the fence lines SHALL be styled as code, the inner line SHALL be styled as code, and the line after the closing fence SHALL return to normal parsing

#### Scenario: Code fence with language tag
- **WHEN** the input line is "```rust"
- **THEN** the line SHALL be treated as a code fence opener and styled as code

### Requirement: Blockquote rendering
The system SHALL detect lines starting with `> ` and style them with italic and a dimmed color.

#### Scenario: Blockquote line
- **WHEN** the input line is "> This is a quote"
- **THEN** the output SHALL be a single Span with italic and dimmed style

### Requirement: List item rendering
The system SHALL detect lines starting with `- `, `* `, or a number followed by `. ` (e.g., `1. `) and style the list marker distinctly from the rest of the line. The marker SHALL be styled with a accent color, and the remaining content SHALL have inline markdown parsing applied.

#### Scenario: Unordered list item
- **WHEN** the input line is "- Buy groceries"
- **THEN** the marker "- " SHALL be styled with accent color, and "Buy groceries" SHALL have inline parsing applied

#### Scenario: Ordered list item
- **WHEN** the input line is "1. First item"
- **THEN** the marker "1. " SHALL be styled with accent color, and "First item" SHALL have inline parsing applied

### Requirement: Bold inline rendering
The system SHALL detect text enclosed in `**` markers and style it as bold. The delimiters SHALL remain visible in the output.

#### Scenario: Bold text
- **WHEN** the input line is "This is **bold** text"
- **THEN** the output SHALL contain three spans: "This is " (unstyled), "**bold**" (bold), " text" (unstyled)

#### Scenario: Unclosed bold marker
- **WHEN** the input line is "This is **not closed"
- **THEN** the text SHALL be rendered without bold styling (treated as plain text)

### Requirement: Italic inline rendering
The system SHALL detect text enclosed in single `*` or `_` markers and style it as italic. The delimiters SHALL remain visible.

#### Scenario: Italic with asterisks
- **WHEN** the input line is "This is *italic* text"
- **THEN** "**italic**" SHALL be styled as italic (with the `*` delimiters visible)

#### Scenario: Italic with underscores
- **WHEN** the input line is "This is _italic_ text"
- **THEN** "_italic_" SHALL be styled as italic

### Requirement: Inline code rendering
The system SHALL detect text enclosed in single backticks and style it with a distinct color. The backtick delimiters SHALL remain visible.

#### Scenario: Inline code
- **WHEN** the input line is "Use the `println!` macro"
- **THEN** "`println!`" SHALL be styled with code color (e.g., Green)

#### Scenario: Unclosed backtick
- **WHEN** the input line is "Use the `println! macro"
- **THEN** the text SHALL be rendered without code styling
