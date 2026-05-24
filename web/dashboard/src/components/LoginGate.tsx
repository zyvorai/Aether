import { useState, useCallback, useEffect } from 'react';
import {
  Hexagon,
  KeyRound,
  LogIn,
  Eye,
  EyeOff,
  Loader2,
  ArrowRight,
  Shield,
  Radio,
  Layers,
  Container,
} from 'lucide-react';
import { apiTryAuth, DEFAULT_DASHBOARD_USERNAME, apiFetchAuthProviders } from '../utils/api';
import { useTheme } from '../contexts/ThemeContext';
import { ZyvorFooter } from './ZyvorBrand';
import {
  PremiumLoginShell,
  LoginError,
  LoginField,
  LoginSubmit,
  LoginDivider,
  LoginRemember,
} from './PremiumLoginShell';

interface LoginGateProps {
  onAuthenticated: (username: string) => void;
}

const REMEMBER_USERNAME_KEY = 'aether_login_username';

const FEATURES = [
  {
    icon: <Layers className="w-5 h-5 text-cyan-100" />,
    title: 'One spec, every runtime',
    description: 'Podman, Kubernetes, KubeVirt, and Metal3 from a single YAML manifest.',
    gradient: 'from-cyan-500/95 to-blue-800/95',
    glow: 'shadow-cyan-500/25',
    highlight: true,
  },
  {
    icon: <Shield className="w-5 h-5 text-emerald-100" />,
    title: 'RBAC & bearer auth',
    description: 'API keys, OIDC, and SAML with fine-grained access control.',
    gradient: 'from-emerald-500/95 to-teal-800/95',
    glow: 'shadow-emerald-500/25',
  },
  {
    icon: <Radio className="w-5 h-5 text-violet-100" />,
    title: 'Live SSE event stream',
    description: 'Watch deployments, health, and runtime transitions in real time.',
    gradient: 'from-violet-500/95 to-fuchsia-800/95',
    glow: 'shadow-violet-500/25',
  },
] as const;

function authErrorMessage(reason: 'network' | 'unauthorized' | 'rejected'): string {
  switch (reason) {
    case 'network':
      return 'Cannot reach the Aether API. Check that the server is running and your network connection.';
    case 'unauthorized':
      return 'Invalid or expired bearer token. Check AETHER_API_KEY or an RBAC key from Access Control.';
    case 'rejected':
      return 'API rejected this key. Check AETHER_API_KEY or an RBAC key from Access Control.';
  }
}

export default function LoginGate({ onAuthenticated }: LoginGateProps) {
  const { theme } = useTheme();
  const loginThemeClass = theme === 'light' ? 'login-page-light' : theme === 'steel' ? 'login-page-steel' : '';
  const [username, setUsername] = useState(() => {
    try {
      return localStorage.getItem(REMEMBER_USERNAME_KEY) ?? DEFAULT_DASHBOARD_USERNAME;
    } catch {
      return DEFAULT_DASHBOARD_USERNAME;
    }
  });
  const [rememberName, setRememberName] = useState(() => {
    try {
      return localStorage.getItem(REMEMBER_USERNAME_KEY) !== null;
    } catch {
      return false;
    }
  });
  const [token, setToken] = useState('');
  const [showToken, setShowToken] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [providersLoading, setProvidersLoading] = useState(true);
  const [oidcEnabled, setOidcEnabled] = useState(false);
  const [samlEnabled, setSamlEnabled] = useState(false);

  useEffect(() => {
    let cancelled = false;
    void apiFetchAuthProviders().then((p) => {
      if (cancelled) return;
      if (p?.oidc?.enabled) setOidcEnabled(true);
      if (p?.saml?.enabled) setSamlEnabled(true);
      setProvidersLoading(false);
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
      const result = await apiTryAuth(token.trim());
      if (!result.ok) {
        setBusy(false);
        setError(authErrorMessage(result.reason));
        return;
      }
      const u = username.trim() || DEFAULT_DASHBOARD_USERNAME;
      try {
        if (rememberName) {
          localStorage.setItem(REMEMBER_USERNAME_KEY, u);
        } else {
          localStorage.removeItem(REMEMBER_USERNAME_KEY);
        }
      } catch {
        /* ignore storage errors */
      }
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
    [token, username, rememberName, onAuthenticated],
  );

  const logo = (
    <div className="w-14 h-14 rounded-2xl flex items-center justify-center bg-gradient-to-br from-cyan-400 via-sky-500 to-blue-700 shadow-xl shadow-cyan-500/40 border border-white/20">
      <Hexagon className="w-7 h-7 text-white" strokeWidth={1.5} aria-hidden />
    </div>
  );

  return (
    <PremiumLoginShell
      accent="cyan"
      pageThemeClass={loginThemeClass}
      productName="Aether"
      productSubtitle="Universal runtime control plane"
      logo={logo}
      heroHeadline={
        <>
          Deploy anywhere
          <br />
          <span className="login-text-gradient">from one YAML spec</span>
        </>
      }
      heroSubheadline="Podman, Kubernetes, KubeVirt, and Metal3 — unified lifecycle, RBAC, and live events without juggling four consoles."
      pills={[
        { icon: <Container className="w-3 h-3" />, label: 'Podman' },
        { icon: <Container className="w-3 h-3" />, label: 'Kubernetes' },
        { label: 'KubeVirt' },
        { label: 'Metal3', glow: true },
      ]}
      features={[...FEATURES]}
      panelTitle="Welcome back"
      panelSubtitle="Authenticate with your API bearer token or organization SSO"
      panelHint={
        <>
          Same token as{' '}
          <code className="text-[11px] px-1 rounded bg-slate-800/80 text-cyan-300/90">
            Authorization: Bearer …
          </code>
          . Leave blank when the server has no API key and OIDC is off.
        </>
      }
      footer={<ZyvorFooter />}
    >
      {providersLoading ? (
        <div className="mb-6 space-y-3" aria-hidden>
          <div className="h-11 rounded-xl bg-slate-800/60 animate-pulse" />
          <div className="h-px bg-slate-700/50" />
        </div>
      ) : (oidcEnabled || samlEnabled) ? (
        <div className="mb-2">
          {oidcEnabled ? (
            <button type="button" onClick={startOidc} className="login-btn-secondary w-full flex items-center justify-center gap-2">
              <LogIn className="h-4 w-4" aria-hidden />
              Sign in with OIDC
            </button>
          ) : null}
          {samlEnabled ? (
            <button
              type="button"
              onClick={startSaml}
              className={`login-btn-secondary w-full flex items-center justify-center gap-2${oidcEnabled ? ' mt-3' : ''}`}
            >
              <LogIn className="h-4 w-4" aria-hidden />
              Sign in with SAML
            </button>
          ) : null}
          <LoginDivider label="or use bearer token" />
        </div>
      ) : null}

      <form onSubmit={submit}>
        {error ? <LoginError message={error} /> : null}

        <div className="space-y-5">
          <LoginField label="Display name" id="login-username">
            <input
              id="login-username"
              type="text"
              value={username}
              onChange={(ev) => setUsername(ev.target.value)}
              className="login-input"
              autoComplete="username"
            />
          </LoginField>

          <LoginField label="API bearer token" id="login-token">
            <KeyRound className="login-field-icon" aria-hidden />
            <input
              id="login-token"
              type={showToken ? 'text' : 'password'}
              value={token}
              onChange={(ev) => {
                setToken(ev.target.value);
                if (error) setError(null);
              }}
              placeholder="Optional in open dev mode"
              className="login-input pr-11 font-mono placeholder:font-sans"
              autoComplete="current-password"
            />
            <button
              type="button"
              onClick={() => setShowToken((v) => !v)}
              className="absolute right-3.5 top-1/2 -translate-y-1/2 text-slate-500 hover:text-slate-300 transition-colors"
              aria-label={showToken ? 'Hide token' : 'Show token'}
            >
              {showToken ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
            </button>
          </LoginField>
        </div>

        <LoginRemember checked={rememberName} onChange={setRememberName} label="Remember display name" />

        <LoginSubmit loading={busy} disabled={busy}>
          {busy ? (
            <>
              <Loader2 className="h-4 w-4 animate-spin relative z-10" />
              <span className="relative z-10">Verifying…</span>
            </>
          ) : (
            <>
              <span className="relative z-10">Continue to dashboard</span>
              <ArrowRight className="h-4 w-4 relative z-10" />
            </>
          )}
        </LoginSubmit>
      </form>
    </PremiumLoginShell>
  );
}
