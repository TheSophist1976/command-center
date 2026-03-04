## Tasks

- [x] Add `due_date: Option<String>` to `SetFields` in `src/nlp.rs` (~line 37). Add `#[serde(skip_serializing_if = "Option::is_none")]` attribute. Update the `parse_response` deserialization — the existing `RawAction` struct's `set` field maps to `SetFields`, so the new field will deserialize automatically.
- [x] Update the system prompt in `build_system_prompt` (~line 100): Add `"due_date":null` to the `set` field in the update action JSON format. Add a note that `due_date` should be in YYYY-MM-DD format and the model should resolve relative dates (like "today", "tomorrow", "next monday") to absolute dates using the provided current date.
- [x] Update the action summary in `src/tui.rs` (~line 991): Add a line for `set_fields.due_date` in the summary formatting, e.g., `if let Some(ref d) = set_fields.due_date { set_parts.push(format!("due_date={}", d)); }`.
- [x] Update `format_update_preview` in `src/tui.rs` (~line 1005): Add a block that shows due date changes. Format current due date as the date string or "none", and new due date from `set_fields.due_date`.
- [x] Update the update execution in `src/tui.rs` (~line 1435): After the existing `priority`/`status`/`tags` application, add handling for `set_fields.due_date`. Parse the string with `NaiveDate::parse_from_str(d, "%Y-%m-%d")`. If parsing fails, show a status error and return without modifying tasks. If empty string, clear the due date. Otherwise set the parsed date.
- [x] Add tests: Test that `parse_response` correctly deserializes an update action with `due_date` in the set fields. Test the system prompt includes `due_date` in the update format.
- [x] Build and run `cargo test` to verify everything compiles and passes.
