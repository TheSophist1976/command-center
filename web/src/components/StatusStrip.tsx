import { useEffect, useRef, useState, type ReactNode } from 'react';
import { ChevronDown, Settings, Terminal } from 'lucide-react';
import type { VersionInfo } from '../types';
import { fetchVersion } from '../api';
import { sectionLabelStyle } from '../styles';
import { statusSummary, type AgentActivity, type ServiceSummary } from '../status';
import { SettingsFooter } from './SettingsFooter';

const COLOR = { running: 'var(--teal)', idle: 'var(--fg-faint)', failing: 'var(--danger)', ok: 'var(--nord14)' };

interface StatusProps {
  activity: AgentActivity[];
  services: ServiceSummary;
  onEditAgent: (name: string) => void;
  onOpenServices: () => void;
  onOpenSettings: () => void;
}

function Dot({ color, pulse }: { color: string; pulse?: boolean }) {
  return <span className={pulse ? 'status-dot-pulse' : undefined} style={{ width: 7, height: 7, flex: 'none', borderRadius: 999, background: color }} />;
}

/** What the strip summarises, in full: each agent and what it is doing, failing services, then settings and updates. */
function StatusPanel({ activity, services, onEditAgent, onOpenServices, onOpenSettings, touch, onDone }: StatusProps & { touch?: boolean; onDone: () => void }) {
  const row = { display: 'flex', alignItems: 'center', gap: 10, minHeight: touch ? 44 : 30, padding: '2px 0', width: '100%', background: 'none', border: 'none', color: 'inherit', font: 'inherit', textAlign: 'left' as const };
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 14, padding: touch ? '4px 20px calc(16px + env(safe-area-inset-bottom, 0px))' : '14px 16px 12px' }}>
      {activity.length > 0 && (
        <div>
          <div style={{ ...sectionLabelStyle, marginBottom: 4 }}>Agents</div>
          {activity.map((a) => (
            <button key={a.name} type="button" style={{ ...row, cursor: 'pointer' }} onClick={() => { onDone(); onEditAgent(a.name); }} title={`Edit ${a.name}`}>
              <Dot color={a.state === 'running' ? COLOR.running : COLOR.idle} pulse={a.state === 'running'} />
              <span style={{ fontFamily: 'var(--font-mono)', fontSize: 13, color: 'var(--fg-2)' }}>{a.name}</span>
              <span style={{ marginLeft: 'auto', minWidth: 0, fontSize: 12, color: 'var(--fg-4)', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                {a.running.length > 0 ? `#${a.running[0].id} ${a.running[0].title}` : 'idle'}
              </span>
            </button>
          ))}
        </div>
      )}
      <div>
        <div style={{ ...sectionLabelStyle, marginBottom: 4 }}>Services</div>
        {services.failing.map((s) => (
          <div key={s.id} style={row}>
            <Dot color={COLOR.failing} />
            <span style={{ fontSize: 13, color: 'var(--fg-2)' }}>{s.title}</span>
            <span style={{ marginLeft: 'auto', fontSize: 12, color: COLOR.failing }}>failing</span>
          </div>
        ))}
        <div style={{ ...row, minHeight: touch ? 36 : 26, color: 'var(--fg-4)', fontSize: 12.5 }}>
          {services.failing.length === 0 ? 'No failing services · ' : ''}{services.ok} {services.ok === 1 ? 'service' : 'services'} ok{services.paused > 0 ? ` · ${services.paused} paused` : ''}
        </div>
        <button type="button" style={{ ...row, cursor: 'pointer', color: 'var(--accent)', fontSize: 13 }} onClick={() => { onDone(); onOpenServices(); }}>
          Open Services…
        </button>
      </div>
      <div style={{ margin: touch ? '0 -2px' : '0 -2px' }}>
        <SettingsFooter touch={touch} onOpenSettings={() => { onDone(); onOpenSettings(); }} />
      </div>
    </div>
  );
}

function summaryChips(activity: AgentActivity[], services: ServiceSummary): ReactNode[] {
  const s = statusSummary(activity, services);
  const chips: ReactNode[] = [];
  if (activity.length > 0) {
    chips.push(
      <span key="agents" style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}>
        <Dot color={s.running > 0 ? COLOR.running : COLOR.idle} pulse={s.running > 0} />
        {s.running} running · {s.idle} idle
      </span>,
    );
  }
  chips.push(
    <span key="services" style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}>
      <Dot color={s.failing > 0 ? COLOR.failing : COLOR.ok} />
      {s.failing > 0 ? `${s.failing} failing` : 'services ok'}
    </span>,
  );
  return chips;
}

function useDismiss(open: boolean, close: () => void, ref: React.RefObject<HTMLElement | null>) {
  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) close();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') close();
    };
    document.addEventListener('mousedown', onDown);
    document.addEventListener('keydown', onKey);
    return () => {
      document.removeEventListener('mousedown', onDown);
      document.removeEventListener('keydown', onKey);
    };
  }, [open, close, ref]);
}

/** Desktop: one thin strip along the top. Agents and services at a glance; click for the detail. */
export function StatusStrip(props: StatusProps) {
  const [open, setOpen] = useState(false);
  const [version, setVersion] = useState<VersionInfo | null>(null);
  const ref = useRef<HTMLDivElement>(null);
  useDismiss(open, () => setOpen(false), ref);
  useEffect(() => {
    fetchVersion().then(setVersion).catch(() => {});
  }, []);

  return (
    <div ref={ref} style={{ position: 'relative', flex: 'none', zIndex: 30 }}>
      <div
        role="status"
        style={{
          display: 'flex', alignItems: 'center', gap: 6, height: 36, padding: '0 12px 0 16px', background: 'var(--ink-2)',
          borderBottom: '1px solid var(--hairline)', fontSize: 12.5, color: 'var(--fg-3)',
        }}
      >
        <span style={{ display: 'flex', alignItems: 'center', gap: 8, fontFamily: 'var(--font-display)', fontWeight: 700, fontSize: 13, color: 'var(--fg-1)', marginRight: 10 }}>
          <span style={{ width: 18, height: 18, borderRadius: 4, background: 'var(--accent)', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
            <Terminal size={11} color="var(--ink)" />
          </span>
          command center
        </span>
        <button
          type="button"
          onClick={() => setOpen((v) => !v)}
          aria-expanded={open}
          aria-label="Agent and service status"
          style={{ display: 'flex', alignItems: 'center', gap: 14, height: 26, padding: '0 8px', borderRadius: 999, border: 'none', background: open ? 'var(--ink-3)' : 'transparent', color: 'inherit', font: 'inherit', cursor: 'pointer' }}
        >
          {summaryChips(props.activity, props.services)}
          <ChevronDown size={13} color="var(--fg-4)" />
        </button>
        <span style={{ flex: 1 }} />
        {version?.update_available && version.latest && (
          <button type="button" onClick={() => setOpen(true)} style={{ border: 'none', background: 'transparent', color: 'var(--accent)', font: 'inherit', cursor: 'pointer' }}>
            v{version.latest} available
          </button>
        )}
        {version && <span style={{ fontFamily: 'var(--font-mono)', fontSize: 11.5, color: 'var(--fg-5)' }}>v{version.current}</span>}
        <button
          type="button"
          onClick={() => props.onOpenSettings()}
          title="Settings"
          aria-label="Settings"
          style={{ display: 'flex', alignItems: 'center', gap: 6, height: 26, padding: '0 8px', borderRadius: 999, border: 'none', background: 'transparent', color: 'var(--fg-3)', font: 'inherit', cursor: 'pointer' }}
        >
          <Settings size={14} color="var(--fg-4)" /> Settings
        </button>
      </div>
      {open && (
        <div
          style={{
            position: 'absolute', top: 38, left: 150, width: 380, background: 'var(--ink-2)', border: '1px solid var(--hairline)',
            borderRadius: 8, boxShadow: '0 12px 30px rgba(0, 0, 0, 0.4)',
          }}
        >
          <StatusPanel {...props} onDone={() => setOpen(false)} />
        </div>
      )}
    </div>
  );
}

/** Phone: the same summary as a slim bar; tapping it opens the detail as a bottom sheet. */
export function MobileStatusBar(props: StatusProps) {
  const [open, setOpen] = useState(false);
  return (
    <>
      <button
        type="button"
        onClick={() => setOpen(true)}
        aria-label="Agent and service status"
        style={{
          display: 'flex', alignItems: 'center', gap: 14, flex: 'none', minHeight: 40, padding: '0 16px', width: '100%',
          background: 'var(--ink-2)', border: 'none', borderBottom: '1px solid var(--hairline)', color: 'var(--fg-3)',
          font: 'inherit', fontSize: 12.5, textAlign: 'left', cursor: 'pointer',
        }}
      >
        {summaryChips(props.activity, props.services)}
        <span style={{ flex: 1 }} />
        <ChevronDown size={14} color="var(--fg-4)" />
      </button>
      {open && (
        <>
          <div onClick={() => setOpen(false)} style={{ position: 'fixed', inset: 0, zIndex: 40, background: 'rgba(0, 0, 0, 0.5)' }} />
          <div
            role="dialog"
            aria-label="Agent and service status"
            style={{
              position: 'fixed', left: 0, right: 0, bottom: 0, zIndex: 41, maxHeight: '80dvh', overflowY: 'auto',
              background: 'var(--ink-2)', borderTop: '1px solid var(--hairline)', borderRadius: '16px 16px 0 0',
            }}
          >
            <div style={{ width: 40, height: 4, borderRadius: 2, background: 'var(--hairline)', margin: '10px auto 12px' }} />
            <StatusPanel {...props} touch onDone={() => setOpen(false)} />
          </div>
        </>
      )}
    </>
  );
}
