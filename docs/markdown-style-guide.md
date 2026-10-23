# Markdown Style Guide

How to write text that reads well in the Command Center web UI. It applies to
humans and agents alike, and to every free-text field.

## Where markdown renders

All text goes through one component, `web/src/components/Markdown.tsx`, which
renders GitHub-flavoured markdown (tables, task lists, strikethrough, bare-URL
autolinks).

| Field | Rendering | Copy button |
|-------|-----------|-------------|
| Task title | Inline only: bold, italics, `code`, links | No |
| Description, instructions | Full markdown | Yes, copies the raw markdown |
| Note bodies | Full markdown | Yes |
| Question notes | Full markdown | Yes |
| Review-thread messages | Full markdown, one chat bubble per `## ` section | Yes, per message |
| Service run summaries | Inline only | No |
| Code blocks (anywhere) | Monospace, wraps long lines | Yes, copies the code |

All links open in a new tab. Clicking a link inside an editable field opens
the link instead of starting the edit.

## Principles

1. **Lead with the answer.** The first line should say what happened or what
   is needed ("PR opened, build green", "Need the API key name"). Details go
   after it.
2. **Short blocks.** Keep paragraphs to 1–3 sentences. Use a list as soon as
   you have three or more parallel items.
3. **One idea per bullet.** Don't nest lists more than one level deep.
4. **Make actionable things copyable.** Commands, paths, ids, branch names and
   note slugs go in `inline code`. Anything multi-line goes in a fenced block.
5. **Make references clickable.** Prefer `[PR #42](https://github.com/…/pull/42)`
   over a bare URL in prose. A bare URL is still clickable.

## Formatting reference

**Section labels.** Use `**Bold label**` on its own line or `###` headings.
In review threads and question notes, do not start a line with `## ` except
the required `## Agent response — YYYY-MM-DD` heading: the UI starts a new
chat bubble at every `## ` line.

**Emphasis.** `**bold**` for the few words a reader must not miss; `_italics_`
sparingly. Don't bold whole sentences.

**Code.**

````markdown
Run `task show 100` to check the note link, then:

```sh
git -C ~/Work/agent-worktrees/CommandCenter-task-100 push -u origin agent/task-100
```
````

Always tag fenced blocks with a language (`sh`, `rust`, `ts`, `json`, `diff`).
Never put a URL in backticks: it stops being a link.

**Checklists.**

```markdown
- [x] Shared Markdown component
- [ ] Mobile screenshot check
```

**Tables.** Only for genuinely tabular data (e.g. a field-by-field comparison).
Keep them to 2–4 columns; wide tables scroll sideways on phones.

**Quotes.** `> ` for quoting a human's request or an error message.

## Example: an agent result note

```markdown
**Summary**
Added markdown rendering to every text field and a copy button to long fields.

**PR:** [#43](https://github.com/TheSophist1976/command-center/pull/43) on branch `agent/task-100`

**Verification**
- `npm run build` passes
- `npm run lint` shows no new warnings

**Open questions**
- Should task titles in the list view render markdown too?
```

## Avoid

- Walls of text without line breaks.
- HTML tags; raw HTML is not rendered.
- Images; notes are files on the server, so relative image paths will not load.
- Headings deeper than `###`; they look the same as `###`.
