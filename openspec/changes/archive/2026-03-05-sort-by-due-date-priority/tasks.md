## 1. Priority Ordering

- [x] 1.1 Derive `PartialOrd` and `Ord` on the `Priority` enum in `src/task.rs`
- [x] 1.2 Add unit tests verifying Priority ordering (Critical < High < Medium < Low by discriminant)

## 2. Sort Implementation

- [x] 2.1 Add a sort step to `filtered_indices()` in `src/tui.rs` that sorts the collected indices by due date ascending (None last) then priority descending
- [x] 2.2 Add unit tests for sort order: mixed due dates sort ascending, None due dates sort last, same due date sorts by priority descending

## 3. Existing Test Fixes

- [x] 3.1 Review and update any existing tests that depend on insertion order of filtered results
