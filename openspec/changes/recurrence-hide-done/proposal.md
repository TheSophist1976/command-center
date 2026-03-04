## Why

The Recurring view currently shows completed tasks alongside open ones. Since completed recurring tasks have already spawned their next occurrence, showing them clutters the view with tasks the user has already finished. The Recurring view should focus on upcoming/active recurring tasks only.

## What Changes

- Filter out completed (done) tasks from the Recurring view so only open recurring tasks are shown
- The All view continues to show all tasks including completed recurring ones

## Capabilities

### New Capabilities
_(none)_

### Modified Capabilities
- `tui-views`: Change the Recurring view filter to exclude done tasks

## Impact

- `src/tui.rs`: Modify the `View::matches` method to exclude done tasks from the Recurring view
