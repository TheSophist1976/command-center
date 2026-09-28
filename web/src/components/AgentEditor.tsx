import { useEffect, useState } from 'react';
import { X } from 'lucide-react';
import type { AgentProfile, Note } from '../types';
import { Button } from './Button';
import {
  fetchAgentInstructions,
  saveAgentInstructions,
  fetchAgentMemory,
  saveAgentMemory,
} from '../api';

interface AgentEditorProps {
  agent: AgentProfile;
  onClose: () => void;
}

type Tab = 'instructions' | 'memory';

function tabLoaders(tab: Tab) {
  return tab === 'instructions'
    ? { fetch: fetchAgentInstructions, save: saveAgentInstructions }
    : { fetch: fetchAgentMemory, save: saveAgentMemory };
}

export function AgentEditor({ agent, onClose }: AgentEditorProps) {
  const [tab, setTab] = useState<Tab>('instructions');
  const [loading, setLoading] = useState(true);
  const [title, setTitle] = useState('');
  const [body, setBody] = useState('');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [dirty, setDirty] = useState(false);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    tabLoaders(tab)
      .fetch(agent.name)
      .then((note: Note | null) => {
        if (cancelled) return;
        setTitle(note?.title ?? `${agent.name} ${tab === 'instructions' ? 'Instructions' : 'Memory'}`);
        setBody(note?.body ?? '');
        setDirty(false);
      })
      .catch((e) => !cancelled && setError(e.message))
      .finally(() => !cancelled && setLoading(false));
    return () => {
      cancelled = true;
    };
  }, [agent.name, tab]);

  async function handleSave() {
    setSaving(true);
    setError(null);
    try {
      await tabLoaders(tab).save(agent.name, { title, body });
      setDirty(false);
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setSaving(false);
    }
  }

  function switchTab(next: Tab) {
    if (next === tab) return;
    if (dirty && !window.confirm('Discard unsaved changes?')) return;
    setTab(next);
  }

  return (
    <div
      onClick={onClose}
      style={{ position: 'fixed', inset: 0, background: 'rgba(0,0,0,0.5)', display: 'flex', alignItems: 'center', justifyContent: 'center', zIndex: 100 }}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        style={{ background: 'var(--ink-2)', border: '1px solid var(--hairline)', borderRadius: 8, padding: 24, width: 560, maxWidth: '90vw', color: 'var(--fg-1)' }}
      >
        <div style={{ display: 'flex', alignItems: 'center', marginBottom: 16 }}>
          <span style={{ flex: 1, fontFamily: 'var(--font-display)', fontWeight: 700, fontSize: 16 }}>
            Edit agent — {agent.name}
          </span>
          <button onClick={onClose} style={{ background: 'transparent', border: 'none', cursor: 'pointer', display: 'flex' }}>
            <X size={16} color="var(--fg-4)" />
          </button>
        </div>

        <div style={{ fontSize: 12, color: 'var(--fg-5)', marginBottom: 16, fontFamily: 'var(--font-mono)' }}>
          {agent.dir}
        </div>

        <div style={{ display: 'flex', gap: 4, marginBottom: 16, borderBottom: '1px solid var(--hairline)' }}>
          {(['instructions', 'memory'] as Tab[]).map((t) => (
            <button
              key={t}
              onClick={() => switchTab(t)}
              style={{
                background: 'transparent',
                border: 'none',
                borderBottom: t === tab ? '2px solid var(--magenta)' : '2px solid transparent',
                color: t === tab ? 'var(--fg-1)' : 'var(--fg-4)',
                fontWeight: t === tab ? 600 : 400,
                fontSize: 13,
                padding: '0 4px 8px',
                cursor: 'pointer',
                textTransform: 'capitalize',
              }}
            >
              {t}
            </button>
          ))}
        </div>

        {loading ? (
          <div style={{ fontSize: 13, color: 'var(--fg-5)', padding: '20px 0' }}>Loading…</div>
        ) : (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
            <input
              value={title}
              onChange={(e) => {
                setTitle(e.target.value);
                setDirty(true);
              }}
              placeholder="Title"
              style={{
                background: 'var(--ink)',
                border: '1px solid var(--hairline)',
                borderRadius: 4,
                color: 'var(--fg-1)',
                fontSize: 13,
                fontWeight: 600,
                padding: '8px 10px',
              }}
            />
            <textarea
              value={body}
              onChange={(e) => {
                setBody(e.target.value);
                setDirty(true);
              }}
              placeholder={tab === 'instructions' ? 'Agent instructions (markdown)…' : 'Agent memory (markdown)…'}
              rows={14}
              style={{
                background: 'var(--ink)',
                border: '1px solid var(--hairline)',
                borderRadius: 4,
                color: 'var(--fg-1)',
                fontSize: 13,
                padding: '10px',
                fontFamily: 'var(--font-mono)',
                resize: 'vertical',
                lineHeight: 1.5,
              }}
            />
          </div>
        )}

        {error && <div style={{ fontSize: 13, color: 'var(--magenta)', marginTop: 10 }}>{error}</div>}

        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8, marginTop: 16 }}>
          <Button variant="secondary" onClick={onClose}>Close</Button>
          <Button onClick={handleSave} disabled={loading || saving || !dirty}>
            {saving ? 'Saving…' : 'Save'}
          </Button>
        </div>
      </div>
    </div>
  );
}
