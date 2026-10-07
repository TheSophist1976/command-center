import { useEffect, useRef, useState } from 'react';
import { Search } from 'lucide-react';
import type { Task } from '../types';
import { sectionLabelStyle } from '../styles';

export interface PaletteCommand {
  label: string;
  kbd?: string;
  run: () => void;
}

interface CommandPaletteProps {
  tasks: Task[];
  commands: PaletteCommand[];
  onPickTask: (task: Task) => void;
  onClose: () => void;
}

interface Item {
  key: string;
  prefix: string;
  label: string;
  kbd?: string;
  run: () => void;
}

/** ⌘K: find a task by title or id, or run any command. Lists every shortcut. */
export function CommandPalette({ tasks, commands, onPickTask, onClose }: CommandPaletteProps) {
  const [query, setQuery] = useState('');
  const [active, setActive] = useState(0);
  const listRef = useRef<HTMLDivElement>(null);

  const q = query.trim().toLowerCase();
  const taskItems: Item[] = tasks
    .filter((t) => !q || t.title.toLowerCase().includes(q) || String(t.id) === q.replace(/^#/, ''))
    .slice(0, q ? 6 : 4)
    .map((t) => ({ key: `t${t.id}`, prefix: `#${t.id}`, label: t.title, run: () => onPickTask(t) }));
  const commandItems: Item[] = commands
    .filter((c) => !q || c.label.toLowerCase().includes(q))
    .map((c) => ({ key: `c${c.label}`, prefix: '›', label: c.label, kbd: c.kbd, run: c.run }));
  const all = [...taskItems, ...commandItems];
  const idx = Math.min(active, Math.max(all.length - 1, 0));

  useEffect(() => {
    listRef.current?.querySelector(`[data-palette-index="${idx}"]`)?.scrollIntoView({ block: 'nearest' });
  }, [idx]);

  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setActive(Math.min(idx + 1, all.length - 1));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setActive(Math.max(idx - 1, 0));
    } else if (e.key === 'Enter') {
      e.preventDefault();
      all[idx]?.run();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      onClose();
    }
  }

  let n = 0;
  const section = (label: string, items: Item[]) =>
    items.length > 0 && (
      <div style={{ display: 'flex', flexDirection: 'column' }}>
        <div style={{ ...sectionLabelStyle, padding: '10px 10px 6px' }}>{label}</div>
        {items.map((item) => {
          const my = n++;
          return (
            <div
              key={item.key}
              data-palette-index={my}
              onClick={item.run}
              onMouseMove={() => my !== idx && setActive(my)}
              style={{
                display: 'flex', alignItems: 'center', gap: 10, height: 38, flex: 'none', padding: '0 10px', borderRadius: 5,
                cursor: 'pointer', background: my === idx ? 'var(--accent-soft)' : 'transparent',
              }}
            >
              <span style={{ width: 28, flex: 'none', fontFamily: 'var(--font-mono)', fontSize: 12, color: 'var(--fg-5)' }}>{item.prefix}</span>
              <span style={{ flex: 1, minWidth: 0, fontSize: 14, color: 'var(--fg-1)', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                {item.label}
              </span>
              {item.kbd && <span style={{ fontFamily: 'var(--font-mono)', fontSize: 11.5, color: 'var(--accent)' }}>{item.kbd}</span>}
            </div>
          );
        })}
      </div>
    );

  return (
    <>
      <div onClick={onClose} style={{ position: 'fixed', inset: 0, background: 'rgba(0, 0, 0, 0.5)', zIndex: 200 }} />
      <div
        role="dialog"
        aria-label="Command palette"
        style={{
          position: 'fixed', left: '50%', top: 110, transform: 'translateX(-50%)', width: 600, maxWidth: 'calc(100vw - 32px)',
          zIndex: 201, background: 'var(--ink-2)', border: '1px solid var(--hairline)', borderRadius: 8,
          boxShadow: '0 16px 48px rgba(0, 0, 0, 0.45)', display: 'flex', flexDirection: 'column', overflow: 'hidden',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: 10, height: 52, padding: '0 16px', borderBottom: '1px solid var(--hairline)' }}>
          <Search size={16} color="var(--fg-4)" />
          <input
            autoFocus
            value={query}
            onChange={(e) => { setQuery(e.target.value); setActive(0); }}
            onKeyDown={handleKeyDown}
            placeholder="Search tasks or type a command…"
            style={{ flex: 1, background: 'transparent', border: 'none', outline: 'none', color: 'var(--fg-1)', fontSize: 15 }}
          />
          <span style={{ fontFamily: 'var(--font-mono)', fontSize: 11, color: 'var(--fg-4)', border: '1px solid var(--hairline)', borderRadius: 4, padding: '2px 6px' }}>
            esc
          </span>
        </div>
        <div ref={listRef} style={{ maxHeight: 460, overflowY: 'auto', padding: 6 }}>
          {section('Tasks', taskItems)}
          {section('Commands', commandItems)}
          {all.length === 0 && (
            <div style={{ padding: 24, textAlign: 'center', fontSize: 14, color: 'var(--fg-5)' }}>No matches.</div>
          )}
        </div>
        <div style={{ display: 'flex', gap: 16, padding: '10px 16px', borderTop: '1px solid var(--hairline)', fontSize: 12, color: 'var(--fg-4)' }}>
          <span>↑↓ navigate</span>
          <span>↵ run</span>
          <span>Shortcuts work outside the palette too</span>
        </div>
      </div>
    </>
  );
}
