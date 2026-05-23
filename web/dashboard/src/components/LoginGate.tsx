import { useState, useCallback, useEffect } from 'react';
import {
  Hexagon,
  KeyRound,
  LogIn,
  Eye,
  EyeOff,
  Loader2,
  AlertCircle,
  Shield,
  Radio,
  Layers,
} from 'lucide-react';
import { apiTryAuth, DEFAULT_DASHBOARD_USERNAME, apiFetchAuthProviders } from '../utils/api';

interface LoginGateProps {
  onAuthenticated: (username: string) => void;
}

const RUNTIMES = [
  { name: 'Podman', color: 'border-blue-500/25 bg-blue-500/10 text-blue-300' },
  { name: 'Kubernetes', color: 'border-emerald-500/25 bg-emerald-500/10 text-emerald-300' },
  { name: 'KubeVirt', color: 'border-purple-500/25 bg-purple-500/10 text-purple-300' },
  { name: 'Metal3', color: 'border-rose-500/25 bg-rose-500/10 text-rose-300' },
] as const;

const FEATURES = [
  { icon: Layers, label: 'One spec, every runtime' },
  { icon: Shield, label: 'RBAC & bearer auth' },
  { icon: Radio, label: 'Live SSE event stream' },
] as const;

export default function LoginGate({ onAuthenticated }: LoginGateProps) {
  const [username, setUsername] = useState(DEFAULT_DASHBOARD_USERNAME);
  const [token, setToken] = useState('');
  const [showToken, setShowToken] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [oidcEnabled, setOidcEnabled] = useState(false);
  const [samlEnabled, setSamlEnabled] = useState(false);

  useEffect(() => {
    let cancelled = false;
    void apiFetchAuthProviders().then((p) => {
      if (cancelled) return;
      if (p?.oidc?.enabled) setOidcEnabled(true);
      if (p?.saml?.enabled) setSamlEnabled(true);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  const startOidc = useCallback(() => {
    const next = `${window.location.pathname}${window.location.search}${window.location.hash}` || '/';
    window.location.assign(`/api/auth/oidc/login?next=${encodeURIComponent(next)}`);
  }, []);

  const startSaml = useCallback(() => {
    const next = `${window.location.pathname}${window.location.search}${window.location.hash}` || '/';
    window.location.assign(`/api/auth/saml/login?next=${encodeURIComponent(next)}`);
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
    <div className="relative min-h-screen overflow-hidden app-shell text-slate-100">
      <div className="pointer-events-none absolute inset-0 steel-grid opacity-60" aria-hidden />
      <div
        className="login-orb login-orb-a pointer-events-none absolute -left-32 top-1/4 h-[28rem] w-[28rem] rounded-full"
        aria-hidden
      />
      <div
        className="login-orb login-orb-b pointer-events-none absolute -right-24 bottom-0 h-[22rem] w-[22rem] rounded-full"
        aria-hidden
      />
      <div
        className="pointer-events-none absolute inset-x-0 top-0 h-px bg-gradient-to-r from-transparent via-aether/40 to-transparent"
        aria-hidden
      />

      <div className="relative z-10 flex min-h-screen flex-col lg:flex-row">
        <aside className="relative hidden flex-1 flex-col justify-between overflow-hidden border-r border-slate-700/40 px-10 py-12 lg:flex xl:px-14">
          <div
            className="pointer-events-none absolute inset-0 bg-gradient-to-br from-aether/8 via-transparent to-cyan-500/5"
            aria-hidden
          />
          <div className="relative">
            <div className="login-logo-ring mb-8 inline-flex items-center justify-center rounded-2xl p-1">
              <div className="flex h-14 w-14 items-center justify-center rounded-xl border border-aether/30 bg-slate-950/80">
                <Hexagon className="h-8 w-8 text-aether" strokeWidth={1.5} />
              </div>
            </div>
            <h1 className="text-4xl font-semibold tracking-tight text-white xl:text-5xl">Aether</h1>
            <p className="mt-3 max-w-md text-lg leading-relaxed text-slate-400">
              Universal Runtime Control Plane — deploy workloads to any backend from a single YAML spec.
            </p>

            <ul className="mt-10 space-y-3">
              {FEATURES.map(({ icon: Icon, label }) => (
                <li key={label} className="flex items-center gap-3 text-sm text-slate-300">
                  <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg border border-slate-600/50 bg-slate-900/60">
                    <Icon className="h-4 w-4 text-aether" aria-hidden />
                  </span>
                  {label}
                </li>
              ))}
            </ul>
          </div>

          <div className="relative mt-12">
            <p className="mb-3 text-[11px] font-medium uppercase tracking-[0.2em] text-slate-500">
              Supported runtimes
            </p>
            <div className="flex flex-wrap gap-2">
              {RUNTIMES.map((rt) => (
                <span
                  key={rt.name}
                  className={`rounded-full border px-3.5 py-1 text-xs font-medium ${rt.color}`}
                >
                  {rt.name}
                </span>
              ))}
            </div>
          </div>
        </aside>

        <main className="flex flex-1 items-center justify-center px-4 py-10 sm:px-8 lg:max-w-xl lg:flex-none xl:max-w-lg">
          <div className="login-card-glow w-full max-w-md animate-slide-up rounded-[28px] border border-slate-700/50 p-8 sm:p-10 surface-panel">
            <div className="mb-8 flex items-center gap-3 lg:hidden">
              <div className="login-logo-ring flex h-12 w-12 items-center justify-center rounded-xl">
                <Hexagon className="h-6 w-6 text-aether" aria-hidden />
              </div>
              <div>
                <h1 className="text-xl font-semibold tracking-tight text-white">Aether</h1>
                <p className="text-[11px] uppercase tracking-[0.18em] text-slate-500">Dashboard sign-in</p>
              </div>
            </div>

            <div className="mb-8 hidden lg:block">
              <h2 className="text-2xl font-semibold tracking-tight text-white">Welcome back</h2>
              <p className="mt-2 text-sm leading-relaxed text-slate-400">
                Authenticate with your API bearer token or organization SSO.
              </p>
            </div>

            {(oidcEnabled || samlEnabled) ? (
              <div className="mb-6">
                {oidcEnabled ? (
                  <button
                    type="button"
                    onClick={startOidc}
                    className="group flex w-full items-center justify-center gap-2.5 rounded-xl border border-slate-600/80 bg-slate-900/70 py-3 text-sm font-medium text-slate-100 transition hover:border-aether/40 hover:bg-slate-800/90 hover:shadow-[0_0_24px_rgba(99,164,255,0.12)]"
                  >
                    <LogIn className="h-4 w-4 text-aether transition group-hover:scale-110" aria-hidden />
                    Sign in with OIDC
                  </button>
                ) : null}
                {samlEnabled ? (
                  <button
                    type="button"
                    onClick={startSaml}
                    className={`group flex w-full items-center justify-center gap-2.5 rounded-xl border border-slate-600/80 bg-slate-900/70 py-3 text-sm font-medium text-slate-100 transition hover:border-aether/40 hover:bg-slate-800/90 hover:shadow-[0_0_24px_rgba(99,164,255,0.12)]${oidcEnabled ? ' mt-3' : ''}`}
                  >
                    <LogIn className="h-4 w-4 text-aether transition group-hover:scale-110" aria-hidden />
                    Sign in with SAML
                  </button>
                ) : null}
                <div className="relative my-6">
                  <div className="absolute inset-0 flex items-center" aria-hidden>
                    <div className="w-full border-t border-slate-700/80" />
                  </div>
                  <p className="relative flex justify-center">
                    <span className="login-divider-label px-3 text-[11px] font-medium uppercase tracking-[0.16em] text-slate-500">
                      or use bearer token
                    </span>
                  </p>
                </div>
              </div>
            ) : null}

            <form onSubmit={submit} className="space-y-5">
              <div>
                <label
                  htmlFor="login-username"
                  className="mb-1.5 block text-[11px] font-medium uppercase tracking-[0.14em] text-slate-500"
                >
                  Display name
                </label>
                <input
                  id="login-username"
                  type="text"
                  value={username}
                  onChange={(ev) => setUsername(ev.target.value)}
                  className="w-full rounded-xl border border-slate-600/80 bg-slate-950/90 px-3.5 py-2.5 text-sm text-slate-100 outline-none transition placeholder:text-slate-600 focus:border-aether/50 focus:ring-2 focus:ring-aether/20"
                  autoComplete="username"
                />
              </div>

              <div>
                <label
                  htmlFor="login-token"
                  className="mb-1.5 block text-[11px] font-medium uppercase tracking-[0.14em] text-slate-500"
                >
                  API bearer token
                </label>
                <div className="relative">
                  <KeyRound
                    className="pointer-events-none absolute left-3.5 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-500"
                    aria-hidden
                  />
                  <input
                    id="login-token"
                    type={showToken ? 'text' : 'password'}
                    value={token}
                    onChange={(ev) => {
                      setToken(ev.target.value);
                      if (error) setError(null);
                    }}
                    placeholder="Optional in open dev mode"
                    className="w-full rounded-xl border border-slate-600/80 bg-slate-950/90 py-2.5 pl-10 pr-11 font-mono text-sm text-slate-100 outline-none transition placeholder:font-sans placeholder:text-slate-600 focus:border-aether/50 focus:ring-2 focus:ring-aether/20"
                    autoComplete="current-password"
                  />
                  <button
                    type="button"
                    onClick={() => setShowToken((v) => !v)}
                    className="absolute right-2 top-1/2 -translate-y-1/2 rounded-lg p-1.5 text-slate-500 transition hover:bg-slate-800 hover:text-slate-300"
                    aria-label={showToken ? 'Hide token' : 'Show token'}
                  >
                    {showToken ? <EyeOff className="h-4 w-4" aria-hidden /> : <Eye className="h-4 w-4" aria-hidden />}
                  </button>
                </div>
              </div>

              {error ? (
                <div
                  role="alert"
                  className="login-shake flex gap-2.5 rounded-xl border border-red-500/25 bg-red-500/10 px-3.5 py-3 text-sm text-red-300"
                >
                  <AlertCircle className="mt-0.5 h-4 w-4 shrink-0 text-red-400" aria-hidden />
                  <span>{error}</span>
                </div>
              ) : null}

              <button
                type="submit"
                disabled={busy}
                className="flex w-full items-center justify-center gap-2 rounded-xl bg-gradient-to-r from-aether to-aether-light py-3 text-sm font-semibold text-slate-950 shadow-[0_4px_24px_rgba(99,164,255,0.35)] transition hover:shadow-[0_6px_32px_rgba(99,164,255,0.45)] disabled:cursor-not-allowed disabled:opacity-55"
              >
                {busy ? (
                  <>
                    <Loader2 className="h-4 w-4 animate-spin" aria-hidden />
                    Verifying…
                  </>
                ) : (
                  'Continue to dashboard'
                )}
              </button>
            </form>

            <p className="mt-6 text-center text-[11px] leading-relaxed text-slate-500 lg:text-left">
              Same token as{' '}
              <code className="rounded bg-slate-800/80 px-1.5 py-0.5 font-mono text-[10px] text-aether/90">
                Authorization: Bearer …
              </code>
              . Leave blank when the server has no API key and OIDC is off.
            </p>

            <div className="mt-8 border-t border-slate-700/50 pt-6 lg:hidden">
              <p className="mb-3 text-center text-[11px] uppercase tracking-[0.16em] text-slate-500">
                Supported runtimes
              </p>
              <div className="flex flex-wrap justify-center gap-2">
                {RUNTIMES.map((rt) => (
                  <span
                    key={rt.name}
                    className={`rounded-full border px-3 py-1 text-xs font-medium ${rt.color}`}
                  >
                    {rt.name}
                  </span>
                ))}
              </div>
            </div>
          </div>
        </main>
      </div>
    </div>
  );
}
