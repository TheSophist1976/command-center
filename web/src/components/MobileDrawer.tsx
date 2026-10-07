import { Terminal } from 'lucide-react';
import type { AgentProfile, Task } from '../types';
import { ALL_FILTER_ITEMS, GROUP_BY_OPTIONS, countForFilter, type DueFilter, type GroupBy } from '../filters';
import { statusColor, statusFor } from '../mockAgentStatus';
import { sectionLabelStyle } from '../styles';
import { SettingsFooter } from './SettingsFooter';

interface MobileDrawerProps {
  tasks: Task[];
  today: Date;
  agents: AgentProfile[];
  filter: DueFilter;
  groupBy: GroupBy;
  onFilter: (f: DueFilter) => void;
  onGroupBy: (g: GroupBy) => void;
  onClose: () => void;
}

export function MobileDrawer({ tasks, today, agents, filter, groupBy, onFilter, onGroupBy, onClose }: MobileDrawerProps) {
  return (
    <>
      <div onClick={onClose} style={{ position: 'fixed', inset: 0, zIndex: 30, background: 'rgba(0, 0, 0, 0.5)' }} />
      <aside
        style={{
          position: 'fixed', left: 0, top: 0, bottom: 0, width: 304, maxWidth: '85vw', zIndex: 31, background: 'var(--ink-2)',
          borderRight: '1px solid var(--hairline)', display: 'flex', flexDirection: 'column',
          padding: 'calc(env(safe-area-inset-top, 0px) + 16px) 0 calc(env(safe-area-inset-bottom, 0px) + 20px)',
          boxShadow: '8px 0 24px rgba(0, 0, 0, 0.35)', overflowY: 'auto',
        }}
      >
        <div style={{ padding: '0 20px 20px', display: 'flex', alignItems: 'center', gap: 10 }}>
          <div style={{ width: 26, height: 26, borderRadius: 5, background: 'var(--accent)', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
            <Terminal size={15} color="var(--ink)" />
          </div>
          <span style={{ flex: 1, fontFamily: 'var(--font-display)', fontWeight: 700, fontSize: 15 }}>command center</span>
        </div>
        <nav style={{ display: 'flex', flexDirection: 'column', padding: '0 10px', gap: 2, marginBottom: 16 }}>
          {ALL_FILTER_ITEMS.map((item) => {
            const active = filter === item.value;
            const Icon = item.icon;
            return (
              <div
                key={item.value}
                onClick={() => onFilter(item.value)}
                style={{
                  display: 'flex', alignItems: 'center', gap: 12, height: 44, padding: '0 12px', borderRadius: 5, cursor: 'pointer',
                  background: active ? 'var(--accent-soft)' : 'transparent',
                  boxShadow: active ? '0 0 0 1px var(--accent-ring)' : 'none',
                }}
              >
                <Icon size={18} color={active ? 'var(--accent)' : 'var(--fg-4)'} />
                <span style={{ flex: 1, fontSize: 15, fontWeight: active ? 600 : 400, color: active ? 'var(--fg-1)' : 'var(--fg-3)' }}>{item.label}</span>
                <span style={{ fontFamily: 'var(--font-mono)', fontSize: 12.5, color: active ? 'var(--accent)' : 'var(--fg-5)' }}>
                  {countForFilter(tasks, today, item.value)}
                </span>
              </div>
            );
          })}
        </nav>
        <div style={{ ...sectionLabelStyle, padding: '0 22px 8px' }}>Group by</div>
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6, padding: '0 20px 18px' }}>
          {GROUP_BY_OPTIONS.map((o) => {
            const active = o.value === groupBy;
            return (
              <button
                key={o.value}
                onClick={() => onGroupBy(o.value)}
                style={{
                  height: 34, padding: '0 12px', borderRadius: 5, fontSize: 13, fontWeight: 600, cursor: 'pointer',
                  border: `1px solid ${active ? 'var(--accent-ring)' : 'var(--hairline)'}`,
                  background: active ? 'var(--accent-soft)' : 'transparent',
                  color: active ? 'var(--fg-1)' : 'var(--fg-3)',
                }}
              >
                {o.label}
              </button>
            );
          })}
        </div>
        {agents.length > 0 && <div style={{ ...sectionLabelStyle, padding: '0 22px 6px' }}>Agents</div>}
        <div style={{ display: 'flex', flexDirection: 'column', padding: '0 10px', marginBottom: 16 }}>
          {agents.map((a) => {
            const status = statusFor(a.name);
            return (
              <div key={a.name} style={{ display: 'flex', alignItems: 'center', gap: 12, minHeight: 44, padding: '4px 12px', borderRadius: 5 }}>
                <span style={{ width: 7, height: 7, flex: 'none', borderRadius: 999, background: statusColor(status.state) }} />
                <div style={{ flex: 1, minWidth: 0, display: 'flex', flexDirection: 'column', gap: 2 }}>
                  <span style={{ fontFamily: 'var(--font-mono)', fontSize: 13, color: 'var(--fg-3)' }}>{a.name}</span>
                  <span style={{ fontSize: 12, color: 'var(--fg-4)', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                    {status.detail ?? status.state}
                  </span>
                </div>
              </div>
            );
          })}
        </div>
        <SettingsFooter touch />
      </aside>
    </>
  );
}
