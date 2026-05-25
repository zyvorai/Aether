import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from 'react';
import { apiFetchSettled } from '../utils/api';
import type { AuthStatus } from '../types/api';

export interface AuthContextValue {
  auth: AuthStatus | null;
  loading: boolean;
  role: string;
  canMutate: boolean;
  canAdmin: boolean;
  isViewer: boolean;
  refresh: () => Promise<void>;
}

const AuthContext = createContext<AuthContextValue>({
  auth: null,
  loading: true,
  role: 'viewer',
  canMutate: false,
  canAdmin: false,
  isViewer: true,
  refresh: async () => {},
});

export function AuthProvider({ enabled, children }: { enabled: boolean; children: ReactNode }) {
  const [auth, setAuth] = useState<AuthStatus | null>(null);
  const [loading, setLoading] = useState(enabled);

  const refresh = useCallback(async () => {
    if (!enabled) {
      setAuth(null);
      setLoading(false);
      return;
    }
    setLoading(true);
    const result = await apiFetchSettled<AuthStatus>('/auth/me');
    setAuth(result.ok ? result.data : null);
    setLoading(false);
  }, [enabled]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const role = auth?.role ?? 'admin';
  const canMutate = role === 'admin' || role === 'operator';
  const canAdmin = role === 'admin';
  const isViewer = !loading && role === 'viewer';

  const value = useMemo(
    () => ({ auth, loading, role, canMutate, canAdmin, isViewer, refresh }),
    [auth, loading, role, canMutate, canAdmin, isViewer, refresh],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth() {
  return useContext(AuthContext);
}
