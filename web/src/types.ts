export type Priority = 'critical' | 'high' | 'medium' | 'low';
export type Effort = 'high' | 'medium' | 'low';
export type Status = 'open' | 'done';

export interface Task {
  id: number;
  title: string;
  status: Status;
  priority: Priority;
  tags: string[];
  created: string;
  updated?: string;
  description?: string;
  instructions?: string;
  due_date?: string;
  project?: string;
  recurrence?: string;
  notes?: string[];
  agent?: string;
  effort?: Effort;
  work_status?: 'todo' | 'in-progress' | 'waiting-for-review' | 'changes-requested' | 'needs-input' | 'complete' | 'failing';
  kind?: 'task' | 'service';
  paused?: boolean;
  /** Lifecycle state derived by the server (src/lifecycle.rs). */
  state?: LifecycleState;
  /** The moves the human may make now; controls are shown from this list. */
  allowed_actions?: LifecycleAction[];
}

export type LifecycleState =
  | 'todo' | 'in-progress' | 'needs-input' | 'waiting-for-review' | 'changes-requested' | 'done'
  | 'scheduled' | 'running' | 'failing';

export type LifecycleAction =
  | 'start' | 'ask' | 'answer' | 'submit' | 'feedback' | 'done' | 'reopen' | 'reset' | 'defer'
  | 'run-ok' | 'run-failed' | 'pause' | 'resume' | 'to-service' | 'to-task';

/** What the conversation composer offers now; the server derives it from the lifecycle. */
export type ThreadSendKind = 'answer' | 'feedback' | 'message';

export interface Thread {
  note: Note | null;
  allowed_action: { send: ThreadSendKind | null; approve: boolean };
  /** The human wrote last and the agent has not answered yet. */
  unread: boolean;
}

export interface ServiceRun {
  id: number;
  task_id: number;
  agent?: string;
  scheduled_for: string;
  ran_at: string;
  result: 'ok' | 'failed';
  summary?: string;
}

export interface Service extends Task {
  last_run?: ServiceRun | null;
  run_count: number;
}

export interface AgentProfile {
  name: string;
  dir: string;
}

export interface Note {
  slug: string;
  title: string;
  body: string;
  updated?: string;
}

export interface VersionInfo {
  current: string;
  latest: string | null;
  update_available: boolean;
  update_supported: boolean;
}

export interface UpdateStatus {
  phase: 'idle' | 'downloading' | 'verifying' | 'installing' | 'restarting' | 'failed';
  error: { kind: string; message: string } | null;
  skipped_files: string[];
}
