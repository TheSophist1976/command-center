import { useEffect, useState } from 'react';
import { Settings } from 'lucide-react';
import type { UpdateStatus, VersionInfo } from '../types';
import { fetchUpdateStatus, fetchVersion, startUpdate } from '../api';
import { Button } from './Button';

const PHASE_TEXT: Record<UpdateStatus['phase'], string> = {
  idle: 'Starting…',
  downloading: 'Downloading…',
  verifying: 'Verifying…',
  installing: 'Installing…',
  restarting: 'Restarting…',
  failed: 'Failed',
};

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

/**
 * Sidebar footer: the Settings row with the running version, plus the
 * in-app updater when a newer release is available.
 */
export function SettingsFooter({ touch }: { touch?: boolean }) {
  const [info, setInfo] = useState<VersionInfo | null>(null);
  const [busy, setBusy] = useState(false);
  const [text, setText] = useState('');
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetchVersion().then(setInfo).catch(() => {});
  }, []);

  async function runUpdate() {
    if (!info?.latest) return;
    const target = info.latest;
    setError(null);
    setBusy(true);
    setText(PHASE_TEXT.idle);

    try {
      await startUpdate();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setBusy(false);
      return;
    }

    // Follow the phases until the server says it is restarting (or the connection drops
    // because it already re-exec'd). Bounded so the loop cannot run forever.
    const phaseDeadline = Date.now() + 60_000;
    while (Date.now() < phaseDeadline) {
      await sleep(700);
      let status: UpdateStatus;
      try {
        status = await fetchUpdateStatus();
      } catch {
        break;
      }
      if (status.phase === 'failed') {
        setError(status.error?.message ?? 'Update failed');
        setBusy(false);
        return;
      }
      if (status.phase === 'idle') {
        // A freshly re-exec'd server reports idle; reload if it is the new version.
        try {
          const v = await fetchVersion();
          if (v.current === target) {
            window.location.reload();
            return;
          }
        } catch {
          break; // server is mid-restart; wait for it below
        }
        setBusy(false); // already up to date
        return;
      }
      setText(PHASE_TEXT[status.phase]);
      if (status.phase === 'restarting') break;
    }

    setText(PHASE_TEXT.restarting);
    const deadline = Date.now() + 60_000;
    while (Date.now() < deadline) {
      await sleep(1000);
      try {
        const v = await fetchVersion();
        if (v.current === target) {
          window.location.reload();
          return;
        }
      } catch {
        // server is mid-restart
      }
    }
    setError('The server did not come back. Restart `task serve` and reload this page.');
    setBusy(false);
  }

  const updatable = !!info?.update_available && !!info.latest;

  return (
    <div style={{ marginTop: 'auto', padding: touch ? '14px 22px 0' : '14px 20px 0', borderTop: '1px solid var(--hairline-soft)', display: 'flex', flexDirection: 'column', gap: 10 }}>
      {updatable && (
        <div style={{ display: 'flex', alignItems: 'center', gap: 10, fontSize: 12, color: 'var(--fg-2)' }}>
          <span style={{ flex: 1 }}>v{info!.latest} available</span>
          {info!.update_supported && (
            <Button size="sm" onClick={runUpdate} disabled={busy}>
              {busy ? text : 'Update'}
            </Button>
          )}
        </div>
      )}
      {error && <div style={{ fontSize: 12, color: 'var(--danger)' }}>{error}</div>}
      <div style={{ display: 'flex', alignItems: 'center', gap: touch ? 12 : 10, minHeight: touch ? 44 : undefined }}>
        <Settings size={touch ? 18 : 16} color="var(--fg-4)" />
        <span style={{ flex: 1, fontSize: touch ? 15 : 14, color: 'var(--fg-3)' }}>Settings</span>
        {info && (
          <span style={{ fontFamily: 'var(--font-mono)', fontSize: touch ? 12 : 11.5, color: 'var(--fg-5)' }}>v{info.current}</span>
        )}
      </div>
    </div>
  );
}
