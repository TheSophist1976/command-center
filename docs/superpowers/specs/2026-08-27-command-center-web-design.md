# Command Center Web

## Problem

The `task` CLI and `task-tui` give a fast, keyboard-driven way to manage tasks stored in `tasks.db` (SQLite), but both require a terminal. A Claude-Design mockup (`Command Center Web.dc.html`, imported from claude.ai/design project `22cad86b-ec0d-42a9-9369-75d9fc346cd3`) explores what a browser-based companion could look like, presenting two alternative shells ("Turn 1 — two shells") for comparison: **1a** ("Rail · Table · Inspector" — a dense sidebar + table + permanent detail panel, close to the TUI's own shape) and **1b** ("Horizon board" — kanban-style lanes by due date with drag-and-drop). This change builds a real, working web frontend based on **1a**, backed by the actual SQLite task store.

## Goals

- A browser-based UI showing real tasks from the user's `tasks.db`, structured as: left rail (due-window/view navigation + agent list) → header (title, search, group-by, new task) → grouped task table → permanent right-side inspector panel.
- Full CRUD against real data: view, add, edit, mark done (with recurrence spawning the next occurrence, matching CLI/TUI behavior), reopen, delete.
- A small new HTTP API (`task_server` binary) that exposes the existing SQLite-backed task store as JSON, reusing `src/db.rs` and `src/task.rs` directly.
- Visual design matching the 1a mockup's structure, colors, and typography — implemented with real, token-based CSS (extracted from `colors_and_type.css`), not literal copied inline styles.

## Non-goals (deferred, not built in this change)

- The 1b "Horizon board" layout, or any drag-and-drop interaction.
- Live agent status (running/waiting/idle). There is currently no cross-process source for this — `src/claude_session.rs` tracks Claude Code child-process status, but only in the memory of whichever TUI process spawned it; a separate `task_server` process cannot read it. Per explicit direction, the agent-status UI (sidebar dots, status strip, "agent waiting" panel) is **mocked with static placeholder data** in this change, visually matching the design but not wired to anything real. A future change will design the actual cross-process status mechanism (this is also the natural first building block of "live agent Q&A," also deferred).
- Live agent Q&A (an agent posting a question on a task, the human replying from the inspector, the agent resuming). The inspector's "Agent waiting" block is part of the same mocked section above.
- In-browser note viewing/editing. Notes remain CLI/TUI-managed; the inspector only shows a real linked-note indicator (from the task's `notes` field) with no view/edit affordance yet.
- Keyboard shortcuts (`a`, `d`, `E`, `A`, `R`, `⏎`, `g`, `/`, `?`, etc.). Click/mouse-driven only for v1.
- Authentication or multi-user support — this is a local, single-user tool.

## Design

### Backend: `task_server` binary

A new binary, `src/bin/task_server.rs`, added as a new `[[bin]]` entry in `Cargo.toml`. It is **not** feature-gated behind `tui` — it depends only on the always-available `db`/`task`/`config` modules. New dependencies: `axum` (HTTP framework) and `tower-http` (static file serving via `ServeDir`), both requiring `tokio` — added as a normal dependency with the `rt-multi-thread`/`macros` features (`reqwest` already pulls in an async runtime transitively for its non-blocking internals, but `task_server` needs `#[tokio::main]` directly, so `tokio` becomes an explicit dependency).

`task_server` talks to the database via `db::load(path)` / `db::save(path, &TaskFile)` directly — the same functions `commands.rs` uses — and serializes `Task` values straight to JSON (they already derive `Serialize`; no new DTO layer needed for reads). It does **not** call into `commands::*`, because those functions return human-formatted `String` messages for terminal output (e.g. `"Created task 1: Buy milk"`), which is the wrong shape for a JSON API. Instead, `task_server` re-implements the same filter/mutate logic against `TaskFile` directly (the same pattern `commands.rs` uses internally — load, mutate in memory, save), returning structured JSON responses.

Endpoints:

| Method | Path | Behavior |
|---|---|---|
| `GET` | `/api/tasks` | List tasks. Query params mirror `commands::ListArgs`: `status`, `agent`, `project`, `tag`, `due_before`. Returns a JSON array of `Task`, sorted the same way `commands::list` sorts (due date ascending, `None` last, then priority). |
| `GET` | `/api/tasks/:id` | Single task detail, or 404. |
| `POST` | `/api/tasks` | Create. Body mirrors `commands::AddArgs` (JSON instead of CLI flags). Returns the created `Task` (with its assigned id). |
| `PATCH` | `/api/tasks/:id` | Edit. Body mirrors `commands::EditArgs`, all fields optional — only fields present are changed. Returns the updated `Task`. |
| `POST` | `/api/tasks/:id/done` | Mark done. Same recurrence-spawn behavior as `commands::done`/`tui::toggle_task_status` — if the task has a recurrence, creates the next occurrence and returns both the completed task and the spawned one (`{ "completed": Task, "spawned": Task \| null }`). |
| `POST` | `/api/tasks/:id/reopen` | Reopen. Returns the updated `Task`. |
| `DELETE` | `/api/tasks/:id` | Delete. Returns 204, or 404 if the id doesn't exist. |
| `GET` | `/api/agents` | Returns configured agent profiles as `[{ "name": string, "dir": string }]`, from `config::list_agent_profiles()` — real data, but names/dirs only, no status (see Non-goals). |

The database path is resolved the same way the CLI does: `db::resolve_file_path(None)` (config `default-dir` → `TASK_FILE` env → `./tasks.db`), with a `--file`/`TASK_FILE` override available identically to the CLI for consistency, since `task_server` is meant to be run from the same task directory as the CLI.

No authentication. Binds to `127.0.0.1` only, not `0.0.0.0` — this is a local tool, not meant to be exposed on a network.

### Serving model

`task_server` serves the built frontend (`web/dist/`) via `tower_http::services::ServeDir`, mounted at `/`, alongside the `/api/*` routes — one binary, one port, for normal day-to-day use (no separate frontend server needed once built). `task_server` listens on `127.0.0.1:4287` by default (arbitrary but fixed, so the Vite dev-server proxy config has a stable target; overridable via a `--port`/`TASK_SERVER_PORT` env var following the same override pattern the CLI uses for `--file`/`TASK_FILE`). During development, the Vite dev server runs separately (`npm run dev` in `web/`) with its dev-server proxy forwarding `/api/*` requests to `http://127.0.0.1:4287`, giving hot-reload for frontend work without rebuilding the Rust binary.

### Frontend: `web/`

A new `web/` directory at the repo root, a standard Vite + React + TypeScript scaffold (`npm create vite@latest web -- --template react-ts`). Structure:

- `web/src/api.ts` — thin fetch wrapper for the `/api/*` endpoints above, typed against a local `Task` TypeScript interface mirroring the Rust struct's JSON shape (`id, title, status, priority, tags, created, updated, description, due_date, project, recurrence, notes, agent, effort`).
- `web/src/components/` — `Sidebar`, `Header`, `TaskTable` (with agent grouping), `TaskRow`, `Inspector`, `Button` (a small native reimplementation — the mockup's only design-system dependency, confirmed to be a trivial ~40-line component wrapping variant/size lookup tables; not worth depending on the actual `_ds_bundle.js`, which requires a global `window.React` the bundle doesn't itself provide).
- `web/src/tokens.css` — design tokens extracted from the design project's `colors_and_type.css`: color custom properties (`--ink`, `--magenta` `#FF0095`, `--cyan` `#0099FF`, `--teal` `#00F3DB`, `--citrine`/warning `#F5C518`, `--danger` `#E5484D`, foreground opacity scale, hairline borders), font stacks (heading = DM Sans, body = Open Sans, mono = `ui-monospace, "SF Mono", Menlo, Consolas, monospace`), spacing scale, and radius scale. Components use these custom properties and small utility/component classes — not literal inline hex/px values copied from the mockup (the mockup hardcodes values that were hand-matched to these tokens; extracting real tokens keeps the UI maintainable and avoids drift).
- Fonts: the design project bundles DM Sans and Open Sans `.ttf` files (`_ds/.../fonts/`) — these get copied into `web/public/fonts/` and declared via `@font-face` in `tokens.css`, rather than depending on the design project being reachable at runtime.
- Icons: the mockup uses `data-lucide` attributes rendered by the `lucide` UMD script at runtime. The real app uses the `lucide-react` npm package instead (standard React usage, no UMD script/global needed).

### Data flow and behavior

- On load, the app fetches `/api/tasks` (unfiltered) and `/api/agents`, groups tasks by `agent` client-side (matching the mockup's grouped-table structure: one group per agent that has tasks, plus an "unassigned" group for tasks with no `agent` field), and renders the left rail's due-window counts (Today/This week/This month/This year/All tasks) computed client-side from the loaded task list's `due_date` values relative to the current date.
- Selecting a row opens that task in the permanent right inspector, reusing the already-loaded `Task` object from the `/api/tasks` list response (no extra fetch on selection) — the list endpoint already returns full `Task` objects, so `GET /api/tasks/:id` exists for direct linking/refresh but isn't needed on every row click.
- Inspector field edits (due date, effort, agent, project, recurrence, tags) call `PATCH /api/tasks/:id` and update local state from the response.
- "Mark done" calls `POST /api/tasks/:id/done`; if the response includes a `spawned` task, both the completed and new task are reflected in the table without a full reload.
- "New task" opens a minimal add form (title required, matching `commands::add`'s only-title-required behavior) and calls `POST /api/tasks`.
- The agent list in the sidebar, the agent-status strip equivalent, and the inspector's "Agent waiting" block are all static/mocked per the Non-goals section — same visual structure as the design, fixed placeholder content, no live data or interactivity beyond what's explicitly listed above.

### Testing

- Backend: `task_server`'s route handlers get integration tests (using `axum`'s test utilities or plain HTTP requests against a spawned test instance) covering each endpoint against a temp `tasks.db`, mirroring the existing `commands.rs` test patterns (setup helper creates a temp dir + db path, seeds data via `db::save` or the `add` endpoint itself).
- Frontend: component-level tests are out of scope for this first pass given the size of the change already (new binary + full frontend); the implementation plan may add a small number of tests for pure logic (e.g. the due-window bucketing function, the agent-grouping function) as a pragmatic minimum, decided at planning time.
