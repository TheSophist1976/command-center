// Bridges the server's WebAuthn options (JSON, binary fields base64url) and
// the browser's navigator.credentials API (ArrayBuffers).

function fromB64url(s: string): ArrayBuffer {
  const b64 = s.replace(/-/g, '+').replace(/_/g, '/') + '='.repeat((4 - (s.length % 4)) % 4);
  const bin = atob(b64);
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  return bytes.buffer;
}

function toB64url(buf: ArrayBuffer): string {
  const bytes = new Uint8Array(buf);
  let bin = '';
  for (const b of bytes) bin += String.fromCharCode(b);
  return btoa(bin).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

interface CredentialDescriptorJSON {
  type: 'public-key';
  id: string;
}

interface CreationOptionsJSON extends Omit<PublicKeyCredentialCreationOptions, 'challenge' | 'user' | 'excludeCredentials'> {
  challenge: string;
  user: { id: string; name: string; displayName: string };
  excludeCredentials?: CredentialDescriptorJSON[];
}

interface RequestOptionsJSON extends Omit<PublicKeyCredentialRequestOptions, 'challenge' | 'allowCredentials'> {
  challenge: string;
  allowCredentials?: CredentialDescriptorJSON[];
}

export function passkeysSupported(): boolean {
  return window.isSecureContext && typeof window.PublicKeyCredential === 'function';
}

/** Turns browser exceptions into something worth showing on the sign-in screen. */
export function describePasskeyError(e: unknown): string {
  if (e instanceof DOMException) {
    if (e.name === 'NotAllowedError') return 'Cancelled, or Face ID didn’t complete. Try again.';
    if (e.name === 'InvalidStateError') return 'This phone already has a passkey for command center. Use Sign in instead.';
    if (e.name === 'SecurityError') return 'Passkeys need the https:// address shown on your computer.';
  }
  return e instanceof Error ? e.message : String(e);
}

export async function createPasskey(options: unknown) {
  const o = options as CreationOptionsJSON;
  const publicKey: PublicKeyCredentialCreationOptions = {
    ...o,
    challenge: fromB64url(o.challenge),
    user: { ...o.user, id: fromB64url(o.user.id) },
    excludeCredentials: (o.excludeCredentials ?? []).map((c) => ({ ...c, id: fromB64url(c.id) })),
  };
  const cred = (await navigator.credentials.create({ publicKey })) as PublicKeyCredential | null;
  if (!cred) throw new Error('No passkey was created.');
  const r = cred.response as AuthenticatorAttestationResponse;
  return {
    id: cred.id,
    rawId: toB64url(cred.rawId),
    type: cred.type,
    response: {
      clientDataJSON: toB64url(r.clientDataJSON),
      attestationObject: toB64url(r.attestationObject),
    },
  };
}

export async function getPasskey(options: unknown) {
  const o = options as RequestOptionsJSON;
  const publicKey: PublicKeyCredentialRequestOptions = {
    ...o,
    challenge: fromB64url(o.challenge),
    allowCredentials: (o.allowCredentials ?? []).map((c) => ({ ...c, id: fromB64url(c.id) })),
  };
  const cred = (await navigator.credentials.get({ publicKey })) as PublicKeyCredential | null;
  if (!cred) throw new Error('No passkey was chosen.');
  const r = cred.response as AuthenticatorAssertionResponse;
  return {
    id: cred.id,
    rawId: toB64url(cred.rawId),
    type: cred.type,
    response: {
      clientDataJSON: toB64url(r.clientDataJSON),
      authenticatorData: toB64url(r.authenticatorData),
      signature: toB64url(r.signature),
      userHandle: r.userHandle ? toB64url(r.userHandle) : null,
    },
  };
}
