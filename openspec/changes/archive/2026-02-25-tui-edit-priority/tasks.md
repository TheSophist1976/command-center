## 1. Mode Enum & Key Dispatch

- [x] 1.1 Add `EditingPriority`, `EditingTitle`, `EditingTags`, `EditingDescription` variants to the `Mode` enum in `src/tui.rs`
- [x] 1.2 Add match arms for the four new `Mode` variants in `handle_key`, routing to the appropriate handlers

## 2. Normal Mode Keybindings

- [x] 2.1 Add `p` arm to `handle_normal`: guard on non-empty `filtered_indices()`, then set `app.mode = Mode::EditingPriority`
- [x] 2.2 Add `e` arm to `handle_normal`: guard on non-empty `filtered_indices()`, pre-populate `input_buffer` with current title, set `app.mode = Mode::EditingTitle`
- [x] 2.3 Add `t` arm to `handle_normal`: guard on non-empty `filtered_indices()`, pre-populate `input_buffer` with `task.tags.join(" ")`, set `app.mode = Mode::EditingTags`
- [x] 2.4 Add `r` arm to `handle_normal`: guard on non-empty `filtered_indices()`, pre-populate `input_buffer` with `task.description.clone().unwrap_or_default()`, set `app.mode = Mode::EditingDescription`

## 3. Priority Picker Handler

- [x] 3.1 Add `handle_priority` function: map `h`/`m`/`l` to `Priority::High`/`Medium`/`Low`, set `task.priority` and `task.updated`, call `app.save()`, return to `Mode::Normal`
- [x] 3.2 Cancel on `Esc` or any unhandled key: return to `Mode::Normal` without mutation

## 4. Text Edit Handlers (InputAction Extension)

- [x] 4.1 Add `EditTitle`, `EditTags`, `EditDescription` variants to the `InputAction` enum
- [x] 4.2 Wire `Mode::EditingTitle`, `Mode::EditingTags`, `Mode::EditingDescription` in `handle_key` to call `handle_input` with the respective `InputAction`
- [x] 4.3 Implement `EditTitle` commit in `handle_input`: trim input, reject if empty (stay in editing mode), otherwise set `task.title`, `task.updated`, save
- [x] 4.4 Implement `EditTags` commit in `handle_input`: split on whitespace, set `task.tags`, `task.updated`, save (empty input clears tags)
- [x] 4.5 Implement `EditDescription` commit in `handle_input`: trim input, set `task.description` to `Some(value)` or `None` if empty, `task.updated`, save

## 5. Footer Rendering

- [x] 5.1 Add `Mode::EditingPriority` arm to `draw_footer`: show `" Set priority: h)igh  m)edium  l)ow  Esc:cancel "`
- [x] 5.2 Add `Mode::EditingTitle` arm to `draw_footer`: show `" Edit title (required): <buffer>_ "`
- [x] 5.3 Add `Mode::EditingTags` arm to `draw_footer`: show `" Edit tags (space-separated): <buffer>_ "`
- [x] 5.4 Add `Mode::EditingDescription` arm to `draw_footer`: show `" Edit description: <buffer>_ "`
- [x] 5.5 Update `Mode::Normal` footer string to include `p:priority  e:edit  t:tags  r:desc`

## 6. Verification

- [x] 6.1 Run `cargo build` and confirm zero errors and zero warnings
- [x] 6.2 Manually smoke-test each editing mode: press the keybinding, confirm pre-population, confirm/cancel, verify the change persists (or doesn't) in the task file
