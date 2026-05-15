import { useState, useCallback, useEffect } from 'react';
import { Hexagon, KeyRound, LogIn } from 'lucide-react';
import { apiTryAuth, DEFAULT_DASHBOARD_USERNAME, apiFetchAuthProviders } from '../utils/api';

interface LoginGateProps {
  onAuthenticated: (username: string) => void;
}

export default function LoginGate({ onAuthenticated }: LoginGateProps) {
  const [username, setUsername] = useState(DEFAULT_DASHBOARD_USERNAME);
  const [token, setToken] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [oidcEnabled, setOidcEnabled] = useState(false);

  useEffect(() => {
    let cancelled = false;
    void apiFetchAuthProviders().then((p) => {
      if (!cancelled && p?.oidc?.enabled) setOidcEnabled(true);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  const startOidc = useCallback(() => {
    const next = `${window.location.pathname}${window.location.search}${window.location.hash}` || '/';
    window.location.assign(`/api/auth/oidc/login?next=${encodeURIComponent(next)}`);
  }, []);

  const submit = useCallback(
    async (e: React.FormEvent) => {
      e.preventDefault();
      setBusy(true);
      setError(null);
      const ok = await apiTryAuth(token.trim());
      if (!ok) {
        setBusy(false);
        setError('API rejected this key. Check AETHER_API_KEY or an RBAC key from Access Control.');
        return;
      }
      const u = username.trim() || DEFAULT_DASHBOARD_USERNAME;
      sessionStorage.setItem(
        'aether_auth',
        JSON.stringify({
          authenticated: true,
          username: u,
          token: token.trim(),
          authMode: 'bearer',
        }),
      );
      setBusy(false);
      onAuthenticated(u);
    },
    [token, username, onAuthenticated],
  );

  return (
    <div className="min-h-screen flex items-center justify-center px-4 app-shell text-slate-100 steel-grid">
      <div className="w-full max-w-md surface-panel rounded-[24px] p-8 shadow-2xl border border-slate-700/50">
        <div className="flex items-center gap-3 mb-6">
          <div className="flex h-12 w-12 items-center justify-center rounded-xl border border-aether/20 bg-aether/10">
            <Hexagon className="w-6 h-6 text-aether" />
          </div>
          <div>
            <h1 className="text-xl font-semibold text-white tracking-tight">Aether</h1>
            <p className="text-xs uppercase tracking-[0.18em] text-slate-500">Dashboard sign-in</p>
          </div>
        </div>
        <p className="text-sm text-slate-400 mb-6 leading-relaxed">
          Paste the same bearer token you use for the API (<code className="text-aether/90">Authorization: Bearer …</code>),
          or use OpenID Connect when the server has OIDC environment variables configured.
          If the server has no <code className="text-slate-300">AETHER_API_KEY</code> and no RBAC keys, you can leave the key blank for open local dev
          (unless OIDC is enabled).
        </p>
        {oidcEnabled ? (
          <div className="mb-6">
            <button
              type="button"
              onClick={startOidc}
              className="w-full flex items-center justify-center gap-2 rounded-xl border border-slate-600 bg-slate-900/80 hover:bg-slate-800 text-slate-100 font-medium py-2.5 text-sm transition"
            >
              <LogIn className="w-4 h-4 text-aether" aria-hidden />
              Sign in with OIDC
            </button>
            <p className="text-[11px] text-slate-500 mt-2 text-center uppercase tracking-wider">or continue with a token</p>
          </div>
        ) : null}
        <form onSubmit={submit} className="space-y-4">
          <div>
            <label className="block text-xs font-medium uppercase tracking-wider text-slate-500 mb-1.5">Display name</label>
            <input
              type="text"
              value={username}
              onChange={(ev) => setUsername(ev.target.value)}
              className="w-full rounded-xl border border-slate-600 bg-slate-950/80 px-3 py-2.5 text-sm text-slate-100 outline-none focus:border-aether/50"
              autoComplete="username"
            />
          </div>
          <div>
            <label className="block text-xs font-medium uppercase tracking-wider text-slate-500 mb-1.5">API bearer token</label>
            <div className="relative">
              <KeyRound className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-500" aria-hidden />
              <input
                type="password"
                value={token}
                onChange={(ev) => setToken(ev.target.value)}
                placeholder="Optional in open dev mode"
                className="w-full rounded-xl border border-slate-600 bg-slate-950/80 pl-10 pr-3 py-2.5 text-sm text-slate-100 outline-none focus:border-aether/50 font-mono"
                autoComplete="current-password"
              />
            </div>
          </div>
          {error ? <p className="text-sm text-red-400">{error}</p> : null}
          <button
            type="submit"
            disabled={busy}
            className="w-full rounded-xl bg-aether/90 hover:bg-aether text-slate-950 font-semibold py-2.5 text-sm transition disabled:opacity-50"
          >
            {busy ? 'Verifying…' : 'Continue'}
          </button>
        </form>
      </div>
    </div>
  );
}
