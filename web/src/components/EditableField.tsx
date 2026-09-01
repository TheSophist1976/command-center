import { useState, type CSSProperties, type ReactNode } from 'react';

interface SelectOption {
  value: string;
  label: string;
}

interface EditableFieldProps {
  value: string;
  display?: ReactNode;
  onSave: (value: string) => void;
  type?: 'text' | 'date' | 'select' | 'textarea';
  options?: SelectOption[];
  placeholder?: string;
  id?: string;
}

const inputStyle: CSSProperties = {
  width: '100%',
  background: 'var(--ink-2)',
  border: '1px solid var(--magenta)',
  borderRadius: 4,
  color: 'var(--fg-1)',
  fontSize: 13,
  fontFamily: 'inherit',
  padding: '3px 6px',
  boxSizing: 'border-box',
};

export function EditableField({ value, display, onSave, type = 'text', options, placeholder, id }: EditableFieldProps) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(value);

  function startEditing() {
    setDraft(value);
    setEditing(true);
  }

  function commit() {
    setEditing(false);
    if (draft !== value) onSave(draft);
  }

  function cancel() {
    setDraft(value);
    setEditing(false);
  }

  if (!editing) {
    return (
      <div id={id} onClick={startEditing} style={{ cursor: 'pointer', minHeight: 20 }} title="Click to edit">
        {display ?? (value ? value : <span style={{ color: 'var(--fg-5)' }}>{placeholder ?? '—'}</span>)}
      </div>
    );
  }

  if (type === 'select' && options) {
    return (
      <select
        autoFocus
        value={draft}
        onChange={(e) => {
          const v = e.target.value;
          setEditing(false);
          if (v !== value) onSave(v);
        }}
        onBlur={() => setEditing(false)}
        style={inputStyle}
      >
        {options.map((o) => (
          <option key={o.value} value={o.value}>
            {o.label}
          </option>
        ))}
      </select>
    );
  }

  if (type === 'textarea') {
    return (
      <textarea
        autoFocus
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => {
          if (e.key === 'Escape') cancel();
        }}
        rows={3}
        style={{ ...inputStyle, resize: 'vertical' }}
      />
    );
  }

  return (
    <input
      autoFocus
      type={type === 'date' ? 'date' : 'text'}
      value={draft}
      onChange={(e) => setDraft(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => {
        if (e.key === 'Enter') commit();
        if (e.key === 'Escape') cancel();
      }}
      placeholder={placeholder}
      style={inputStyle}
    />
  );
}

export function FieldRow({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div style={{ display: 'flex', alignItems: 'center', minHeight: 32, borderBottom: '1px solid var(--hairline-soft)', padding: '4px 0' }}>
      <span style={{ width: 90, flex: 'none', fontSize: 12.5, color: 'var(--fg-4)' }}>{label}</span>
      <div style={{ flex: 1, fontSize: 13, color: 'var(--fg-1)' }}>{children}</div>
    </div>
  );
}
