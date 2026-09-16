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
  due_date?: string;
  project?: string;
  recurrence?: string;
  notes?: string[];
  agent?: string;
  effort?: Effort;
  work_status?: 'todo' | 'in-progress' | 'waiting-for-review' | 'changes-requested' | 'complete';
}

export interface AgentProfile {
  name: string;
  dir: string;
}

export interface Note {
  slug: string;
  title: string;
  body: string;
}
