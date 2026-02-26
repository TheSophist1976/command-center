## Context

The TUI is built around a modal interaction model in `src/tui.rs`. The current `Mode` enum has four variants: `Normal`, `Adding`, `Filtering`, and `Confirming`. Each mode is handled by a dedicated function, and the footer renders mode-specific help text.

All four target fields (`priority`, `title`, `tags`, `description`) are already first-class fields on `Task`. Priority is a three-variant enum; the others are `String` / `Vec<String>` / `Option<String>`. The storage layer is straightforward: `app.save()` persists the full task file after any mutation.

The existing `Adding` and `Filtering` modes use a shared `handle_input` function with an `InputAction` enum to multiplex behavior. The `Confirming` mode is a single-keystroke response with no buffer.

## Goals / Non-Goals

**Goals:**

- `p` in Normal mode → `EditingPriority`: single-keystroke picker (`h`/`m`/`l`) in footer
- `e` in Normal mode → `EditingTitle`: text input pre-populated with current title
- `t` in Normal mode → `EditingTags`: text input pre-populated with current tags (space-separated)
- `r` in Normal mode → `EditingDescription`: text input pre-populated with current description (empty string if `None`)
- Persist each change immediately on Enter, cancel on Esc
- Update Normal mode footer to include all four new keybindings
- Guard all four keybindings: no-op when no task is selected

**Non-Goals:**

- No multi-line description editing — single-line input in the footer only
- No visual popup or overlay widgets
- No change to CLI behavior or the `task edit` command

## Decisions

### Four new `Mode` variants, not a generic "editing" sub-state

Each field has a distinct input shape (picker vs. text) and distinct commit logic. Separate variants keep `handle_key`, `draw_footer`, and the commit handlers readable and independently testable.

**Alternative considered**: A single `Editing(Field)` variant parameterized by an enum. Rejected — it adds indirection without reducing code, and Rust pattern matching on parameterized variants in `draw_footer` is less readable.

### Priority: single-keystroke picker in the footer

`EditingPriority` matches `Confirming` in structure: no input buffer, handled by a dedicated `handle_priority` function, exits to `Normal` on any key. Footer shows: `Set priority: h)igh  m)edium  l)ow  Esc:cancel`.

**Alternative considered**: Cycle priority directly with `p` (no sub-mode). Rejected — gives no affordance about available options or current value.

### Title / Tags / Description: extend `InputAction`, pre-populate `input_buffer`

These three fields share the text-input shape already used by `Adding` and `Filtering`. Extending `InputAction` with three new variants (`EditTitle`, `EditTags`, `EditDescription`) reuses `handle_input` for all keystroke handling. The only new logic is: on entering the mode, pre-populate `app.input_buffer` with the current field value.

Tag pre-population: `task.tags.join(" ")` — matches the space-separated syntax already used by the filter parser.

Description pre-population: `task.description.clone().unwrap_or_default()`.

On Enter, commit:

- `EditTitle`: trim, reject if empty (a task must have a title), set `task.title`, `task.updated`, save
- `EditTags`: split on whitespace, set `task.tags`, `task.updated`, save (empty input → clears all tags)
- `EditDescription`: trim, set `task.description` to `Some(value)` or `None` if empty, `task.updated`, save

### Keybinding choices

| Key | Field       | Rationale                                                                    |
|-----|-------------|------------------------------------------------------------------------------|
| `p` | priority    | mnemonic: **p**riority                                                       |
| `e` | title       | mnemonic: **e**dit (most intuitive general edit key; `t` is taken by tags)   |
| `t` | tags        | mnemonic: **t**ags                                                           |
| `r` | description | mnemonic: desc**r**iption; `d` is taken by delete                            |

All four keys are currently unbound in Normal mode.

## Risks / Trade-offs

- **Title rejection on empty input**: Differs from `Adding` (which also rejects empty), but slightly surprising if a user clears the buffer and hits Enter. → Mitigation: footer hint shows `(required)` or similar.
- **Single-line description**: Long descriptions are truncated to one line in the footer input. Not ideal for rich descriptions but consistent with the TUI's minimal footprint. → Mitigation: out of scope; full description editing belongs in an editor integration.
- **`r` for description is non-obvious**: Less intuitive than `D` or `i`. → Mitigation: footer help text makes it explicit; mnemonic ("desc**r**iption") is discoverable.
- **No undo**: All changes persist immediately. Consistent with existing TUI behavior. → Mitigation: none planned.

## Migration Plan

1. Add `EditingPriority`, `EditingTitle`, `EditingTags`, `EditingDescription` to the `Mode` enum
2. Add `EditTitle`, `EditTags`, `EditDescription` to `InputAction`
3. Add `p`, `e`, `t`, `r` arms to `handle_normal`, each guarded by a non-empty `filtered_indices()` check; pre-populate `input_buffer` for text modes
4. Add `handle_priority` function for the picker sub-mode
5. Extend `handle_input` commit logic for the three new `InputAction` variants
6. Wire new `Mode` variants into `handle_key`
7. Add new `Mode` arms to `draw_footer` with appropriate prompts
8. Update `Mode::Normal` footer string to include all four new keybindings

No data or storage format changes. Rollback: revert `src/tui.rs`.
