import { useCallback, useEffect, useState, type ReactNode } from 'react';
import { fetchAuthStatus, signOut as apiSignOut, UNAUTHORIZED_EVENT } from './api';
import type { AuthStatus } from './types';
import { SignInScreen } from './components/SignInScreen';
import { AuthContext } from './authContext';

function takePairCode(): string | null {
  return new URLSearchParams(window.location.search).get('pair');
}

function clearPairCode() {
  const url = new URL(window.location.href);
  if (!url.searchParams.has('pair')) return;
  url.searchParams.delete('pair');
  window.history.replaceState(null, '', url.pathname + url.search + url.hash);
}

/**
 * Renders the app once the browser is allowed in: immediately on the desktop,
 * after a passkey sign-in (or pairing, via `?pair=CODE`) on a phone. Any 401
 * from the API afterwards — e.g. the device was revoked — returns here.
 */
export function AuthGate({ children }: { children: ReactNode }) {
  const [status, setStatus] = useState<AuthStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [pairCode, setPairCode] = useState<string | null>(takePairCode);

  const refresh = useCallback(() => {
    fetchAuthStatus()
      .then((s) => {
        setStatus(s);
        setError(null);
        if (s.authenticated) {
          clearPairCode();
          setPairCode(null);
        }
      })
      .catch((e) => setError(String(e)));
  }, []);

  useEffect(refresh, [refresh]);

  useEffect(() => {
    const onUnauthorized = () => setStatus((s) => (s && !s.local ? { ...s, authenticated: false } : s));
    window.addEventListener(UNAUTHORIZED_EVENT, onUnauthorized);
    return () => window.removeEventListener(UNAUTHORIZED_EVENT, onUnauthorized);
  }, []);

  const signOut = useCallback(async () => {
    await apiSignOut().catch(() => {});
    setStatus((s) => (s ? { ...s, authenticated: false } : s));
  }, []);

  if (!status) {
    return error ? <div style={{ padding: 24, color: 'var(--danger)' }}>{error}</div> : null;
  }
  if (!status.authenticated) {
    return <SignInScreen pairCode={pairCode} onSignedIn={refresh} />;
  }
  return (
    <AuthContext.Provider value={{ local: status.local, remoteHost: status.remote_host, signOut }}>
      {children}
    </AuthContext.Provider>
  );
}
