import { useState } from 'react';
import { ScanFace, Terminal } from 'lucide-react';
import { loginOptions, loginVerify, registerOptions, registerVerify } from '../api';
import { createPasskey, describePasskeyError, getPasskey, passkeysSupported } from '../webauthn';

function guessDeviceName(): string {
  const ua = navigator.userAgent;
  if (/iPhone/.test(ua)) return 'iPhone';
  if (/iPad/.test(ua)) return 'iPad';
  if (/Android/.test(ua)) return 'Android phone';
  return 'Phone';
}

function formatCode(code: string): string {
  const c = code.toUpperCase();
  return c.length === 8 ? `${c.slice(0, 4)}-${c.slice(4)}` : c;
}

interface SignInScreenProps {
  /** From `?pair=CODE`: show pairing instead of sign-in. */
  pairCode: string | null;
  onSignedIn: () => void;
}

/** Full-screen sign-in for a phone reaching the server through `remote-host`. */
export function SignInScreen({ pairCode, onSignedIn }: SignInScreenProps) {
  const [pairing, setPairing] = useState(!!pairCode);
  const [name, setName] = useState(guessDeviceName);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const supported = passkeysSupported();

  async function run(fn: () => Promise<void>) {
    setBusy(true);
    setError(null);
    try {
      await fn();
      onSignedIn();
    } catch (e) {
      setError(describePasskeyError(e));
    } finally {
      setBusy(false);
    }
  }

  const pair = () => run(async () => {
    const options = await registerOptions(pairCode!);
    const credential = await createPasskey(options);
    await registerVerify(pairCode!, name, credential);
  });

  const signIn = () => run(async () => {
    const options = await loginOptions();
    const credential = await getPasskey(options);
    await loginVerify(credential);
  });

  return (
    <div
      style={{
        minHeight: '100dvh', display: 'flex', flexDirection: 'column', justifyContent: 'center', gap: 28,
        padding: 'calc(env(safe-area-inset-top, 0px) + 32px) 24px calc(env(safe-area-inset-bottom, 0px) + 32px)',
        maxWidth: 420, margin: '0 auto',
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
        <div style={{ width: 32, height: 32, borderRadius: 6, background: 'var(--accent)', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
          <Terminal size={18} color="var(--ink)" />
        </div>
        <span style={{ fontFamily: 'var(--font-display)', fontWeight: 700, fontSize: 17 }}>command center</span>
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
        <div style={{ fontFamily: 'var(--font-display)', fontWeight: 600, fontSize: 26, lineHeight: 1.2 }}>
          {pairing ? 'Set up Face ID on this phone' : 'Sign in'}
        </div>
        <div style={{ fontSize: 15, lineHeight: 1.5, color: 'var(--fg-4)', textWrap: 'pretty' }}>
          {pairing ? (
            <>
              Pairing code <span style={{ fontFamily: 'var(--font-mono)', color: 'var(--fg-1)' }}>{formatCode(pairCode!)}</span> from
              your computer. This creates a passkey for command center on this phone; you'll sign in with Face ID from now on.
            </>
          ) : (
            'Use the passkey you set up on this phone.'
          )}
        </div>
      </div>

      {!supported ? (
        <div style={{ fontSize: 14, lineHeight: 1.5, color: 'var(--citrine)' }}>
          This browser can’t use passkeys here. Open the https:// address shown on your computer (Settings → Devices) in Safari or Chrome.
        </div>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
          {pairing && (
            <label style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
              <span style={{ fontSize: 12, color: 'var(--fg-4)' }}>Name this device</span>
              <input
                value={name}
                onChange={(e) => setName(e.target.value)}
                maxLength={60}
                style={{
                  height: 48, padding: '0 14px', borderRadius: 5, border: '1px solid var(--hairline)',
                  background: 'var(--ink-2)', color: 'var(--fg-1)', fontSize: 16,
                }}
              />
            </label>
          )}
          <button
            onClick={pairing ? pair : signIn}
            disabled={busy || (pairing && !name.trim())}
            style={{
              height: 52, borderRadius: 8, border: 'none', cursor: busy ? 'wait' : 'pointer', opacity: busy ? 0.6 : 1,
              background: 'var(--accent)', color: 'var(--ink)', fontSize: 17, fontWeight: 600,
              display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 10,
            }}
          >
            <ScanFace size={22} />
            {busy ? 'Waiting for Face ID…' : pairing ? 'Set up Face ID' : 'Sign in with Face ID'}
          </button>
          {error && <div role="alert" style={{ fontSize: 14, lineHeight: 1.5, color: 'var(--danger)' }}>{error}</div>}
        </div>
      )}

      <div style={{ fontSize: 13, lineHeight: 1.5, color: 'var(--fg-5)' }}>
        {pairing ? (
          <button onClick={() => { setPairing(false); setError(null); }} style={linkStyle}>
            Already set up? Sign in instead
          </button>
        ) : (
          <>New phone? On your computer, open <b style={{ color: 'var(--fg-3)' }}>Settings → Devices → Pair a phone</b> and scan the QR code.</>
        )}
      </div>
    </div>
  );
}

const linkStyle = {
  background: 'transparent', border: 'none', padding: 0, cursor: 'pointer', color: 'var(--accent)', fontSize: 13,
} as const;
