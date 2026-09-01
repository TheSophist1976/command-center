import { useState } from 'react';
import { Button } from './Button';

interface NewTaskFormProps {
  onSubmit: (title: string) => void;
  onCancel: () => void;
}

export function NewTaskForm({ onSubmit, onCancel }: NewTaskFormProps) {
  const [title, setTitle] = useState('');

  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        if (title.trim()) onSubmit(title.trim());
      }}
      style={{ display: 'flex', gap: 8, alignItems: 'center' }}
    >
      <input
        autoFocus
        value={title}
        onChange={(e) => setTitle(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'Escape') onCancel();
        }}
        placeholder="Task title"
        style={{
          height: 34,
          padding: '0 12px',
          borderRadius: 5,
          border: '1px solid var(--hairline)',
          background: 'var(--ink-2)',
          color: 'var(--fg-1)',
          fontSize: 14,
          width: 280,
        }}
      />
      <Button size="sm" type="submit">Add</Button>
      <Button size="sm" variant="secondary" onClick={onCancel}>Cancel</Button>
    </form>
  );
}
