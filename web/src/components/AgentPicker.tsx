import { useEffect, useRef, useState } from 'react';

interface Option {
  value: string;
  label: string;
}

interface AgentPickerProps {
  id?: string;
  value: string;
  options: Option[];
  onSave: (value: string) => void;
}

export function AgentPicker({ id, value, options, onSave }: AgentPickerProps) {
  const [open, setOpen] = useState(false);
  const [highlighted, setHighlighted] = useState(0);
  const containerRef = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  function openPicker() {
    const idx = options.findIndex((o) => o.value === value);
    setHighlighted(idx === -1 ? 0 : idx);
    setOpen(true);
  }

  function choose(v: string) {
    onSave(v);
    setOpen(false);
  }

  useEffect(() => {
    if (!open) return;
    listRef.current?.focus();

    function handleClickOutside(e: MouseEvent) {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    }
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [open]);

  useEffect(() => {
    if (!open) return;
    listRef.current?.querySelector(`[data-option-index="${highlighted}"]`)?.scrollIntoView({ block: 'nearest' });
  }, [open, highlighted]);

  function handleKeyDown(e: React.KeyboardEvent) {
    // Every handled key must stop propagation — otherwise the app's global
    // keyboard-shortcut listener on `window` also sees it (e.g. Enter would
    // both confirm the pick AND toggle the selected task's done state).
    if (e.key === 'ArrowDown' || e.key === 'j') {
      e.preventDefault();
      e.stopPropagation();
      setHighlighted((h) => Math.min(h + 1, options.length - 1));
    } else if (e.key === 'ArrowUp' || e.key === 'k') {
      e.preventDefault();
      e.stopPropagation();
      setHighlighted((h) => Math.max(h - 1, 0));
    } else if (e.key === 'Enter') {
      e.preventDefault();
      e.stopPropagation();
      choose(options[highlighted].value);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      setOpen(false);
    } else {
      // Swallow everything else too, so other global shortcuts (e/d/p/t/g/a...)
      // can't fire behind this picker while it's open.
      e.stopPropagation();
    }
  }

  const current = options.find((o) => o.value === value);

  return (
    <div ref={containerRef} style={{ position: 'relative' }}>
      <div
        id={id}
        onClick={openPicker}
        style={{ cursor: 'pointer', minHeight: 20 }}
        title="Click to select"
      >
        {current && current.value ? current.label : <span style={{ color: 'var(--fg-5)' }}>Unassigned</span>}
      </div>
      {open && (
        <div
          ref={listRef}
          tabIndex={-1}
          onKeyDown={handleKeyDown}
          style={{
            position: 'absolute', top: '100%', left: 0, zIndex: 50, marginTop: 4,
            background: 'var(--ink-2)', border: '1px solid var(--hairline)', borderRadius: 6,
            minWidth: 160, maxHeight: 220, overflowY: 'auto', boxShadow: '0 4px 16px rgba(0,0,0,0.4)',
            outline: 'none', padding: 4,
          }}
        >
          {options.map((o, i) => (
            <div
              key={o.value}
              data-option-index={i}
              onMouseEnter={() => setHighlighted(i)}
              onClick={() => choose(o.value)}
              style={{
                padding: '6px 10px', fontSize: 13, cursor: 'pointer', borderRadius: 4,
                background: i === highlighted ? 'rgba(255,0,149,0.15)' : 'transparent',
                color: i === highlighted ? 'var(--fg-1)' : 'var(--fg-3)',
              }}
            >
              {o.label}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
