import './tokens.css';
import { Terminal, Sun, CalendarDays, Settings } from 'lucide-react';
import { Button } from './components/Button';

export default function App() {
  return (
    <div style={{ display: 'flex', height: '100vh', overflow: 'hidden' }}>
      <aside
        style={{
          width: 252,
          flex: 'none',
          background: 'var(--ink-2)',
          borderRight: '1px solid var(--hairline)',
          display: 'flex',
          flexDirection: 'column',
          padding: '20px 0',
        }}
      >
        <div style={{ padding: '0 20px 22px', display: 'flex', alignItems: 'center', gap: 10 }}>
          <div
            style={{
              width: 26,
              height: 26,
              borderRadius: 5,
              background: 'var(--magenta)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
            }}
          >
            <Terminal size={15} color="var(--ink)" />
          </div>
          <span style={{ fontFamily: 'var(--font-display)', fontWeight: 700, fontSize: 14 }}>
            command center
          </span>
        </div>
        <nav style={{ display: 'flex', flexDirection: 'column', padding: '0 10px', gap: 2 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 10, height: 36, padding: '0 10px' }}>
            <Sun size={16} color="var(--magenta)" />
            <span style={{ flex: 1, fontSize: 14 }}>Today</span>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 10, height: 36, padding: '0 10px' }}>
            <CalendarDays size={16} color="var(--fg-4)" />
            <span style={{ flex: 1, fontSize: 14, color: 'var(--fg-3)' }}>This week</span>
          </div>
        </nav>
        <div style={{ marginTop: 'auto', padding: '14px 20px 0', borderTop: '1px solid var(--hairline-soft)', display: 'flex', alignItems: 'center', gap: 10 }}>
          <Settings size={16} color="var(--fg-4)" />
          <span style={{ flex: 1, fontSize: 14, color: 'var(--fg-3)' }}>Settings</span>
        </div>
      </aside>

      <main style={{ flex: 1, minWidth: 0, display: 'flex', flexDirection: 'column' }}>
        <header
          style={{
            height: 72,
            flex: 'none',
            padding: '0 24px',
            borderBottom: '1px solid var(--hairline)',
            display: 'flex',
            alignItems: 'center',
            gap: 16,
          }}
        >
          <span style={{ fontFamily: 'var(--font-display)', fontWeight: 600, fontSize: 22 }}>Today</span>
          <div style={{ flex: 1 }} />
          <Button>New task</Button>
        </header>
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 16,
            height: 34,
            flex: 'none',
            padding: '0 24px',
            borderBottom: '1px solid var(--hairline-soft)',
            fontFamily: 'var(--font-display)',
            fontSize: 11,
            fontWeight: 700,
            letterSpacing: '0.06em',
            textTransform: 'uppercase',
            color: 'var(--fg-5)',
          }}
        >
          <span style={{ width: 34 }}>ID</span>
          <span style={{ width: 78 }}>Priority</span>
          <span style={{ flex: 1 }}>Task</span>
          <span style={{ width: 96 }}>Due</span>
          <span style={{ width: 44 }}>Effort</span>
        </div>
        <div style={{ flex: 1, minHeight: 0, display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'var(--fg-5)' }}>
          (tasks load here — Task 9)
        </div>
      </main>

      <aside
        style={{
          width: 352,
          flex: 'none',
          background: 'var(--ink-3)',
          borderLeft: '1px solid var(--hairline)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          color: 'var(--fg-5)',
        }}
      >
        Select a task
      </aside>
    </div>
  );
}
