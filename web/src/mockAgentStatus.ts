export interface MockAgentStatus {
  name: string;
  state: 'running' | 'waiting' | 'idle';
  detail?: string;
}

// Placeholder data — not backed by a real status source yet (see spec Non-goals:
// live agent status requires a cross-process mechanism not built in this change).
export const MOCK_AGENT_STATUSES: MockAgentStatus[] = [
  { name: 'obsidian-sync', state: 'running', detail: 'rewrote 41 of 128 notes' },
  { name: 'command-center', state: 'waiting', detail: 'asked a question' },
];

export function statusFor(agentName: string): MockAgentStatus {
  return (
    MOCK_AGENT_STATUSES.find((s) => s.name === agentName) ?? { name: agentName, state: 'idle' }
  );
}

const STATE_COLOR: Record<MockAgentStatus['state'], string> = {
  running: 'var(--teal)',
  waiting: 'var(--citrine)',
  idle: 'var(--fg-faint)',
};

export function statusColor(state: MockAgentStatus['state']): string {
  return STATE_COLOR[state];
}
