import { createContext, useContext } from 'react';

interface AuthContextValue {
  /** Browser on the computer running the server: no sign-in, can pair devices. */
  local: boolean;
  remoteHost: string | null;
  signOut: () => Promise<void>;
}

export const AuthContext = createContext<AuthContextValue>({ local: true, remoteHost: null, signOut: async () => {} });

export function useAuth(): AuthContextValue {
  return useContext(AuthContext);
}
