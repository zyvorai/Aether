// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';
import {
  ArrowRight,
  Boxes,
  ChevronDown,
  Container,
  Eye,
  EyeOff,
  Hexagon,
  KeyRound,
  Layers,
  Loader2,
  LockKeyhole,
  LogIn,
  Radio,
  Server,
  ShieldCheck,
  User,
} from 'lucide-react';
import {
  apiFetchAuthProviders,
  apiFetchHealth,
  apiLdapLogin,
  apiLocalLogin,
  apiTryAuth,
  DEFAULT_DASHBOARD_USERNAME,
  type AuthProvidersPayload,
  type HealthPayload,
} from '../utils/api';
import { ZyvorFooter, ZYVOR_URL, ZYVOR_ORANGE } from './ZyvorBrand';

interface LoginGateProps {
  onAuthenticated: (username: string) => void;
  /** Shown above the form, e.g. when the session was dropped by a server-side auth change. */
  notice?: string;
}

type LoginErrorReason = 'network' | 'unauthorized' | 'rejected';

function loginError(reason: LoginErrorReason, method = 'username or password'): string {
  if (reason === 'network') {
    return 'Cannot reach the Aether API. Check that the server is running and try again.';
  }
  if (reason === 'unauthorized') {
    return `Invalid ${method}.`;
  }
  return 'Aether could not complete sign-in. Please try again.';
}

function saveSession(username: string, authMode: 'bearer' | 'cookie', token?: string): void {
  sessionStorage.setItem(
    'aether_auth',
    JSON.stringify({
      authenticated: true,
      username,
      ...(token ? { token } : {}),
      authMode,
    }),
  );
}

export default function LoginGate({ onAuthenticated, notice }: LoginGateProps) {
  const [username, setUsername] = useState(DEFAULT_DASHBOARD_USERNAME);
  const [password, setPassword] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [providers, setProviders] = useState<AuthProvidersPayload | null>(null);
  const [providersLoading, setProvidersLoading] = useState(true);
  const [health, setHealth] = useState<HealthPayload | null>(null);

  const [showLdap, setShowLdap] = useState(false);
  const [ldapPassword, setLdapPassword] = useState('');
  const [ldapBusy, setLdapBusy] = useState(false);
  const [ldapError, setLdapError] = useState<string | null>(null);

  const [showApiKey, setShowApiKey] = useState(false);
  const [apiKey, setApiKey] = useState('');
  const [showApiKeyValue, setShowApiKeyValue] = useState(false);
  const [apiKeyBusy, setApiKeyBusy] = useState(false);
  const [apiKeyError, setApiKeyError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    void apiFetchAuthProviders().then((available) => {
      if (cancelled) return;
      setProviders(available);
      if (available?.local?.default_username) {
        setUsername(available.local.default_username);
      }
      setProvidersLoading(false);
    });
    void apiFetchHealth().then((payload) => {
      if (!cancelled) setHealth(payload);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  const submitLocal = useCallback(
    async (event: React.FormEvent) => {
      event.preventDefault();
      setBusy(true);
      setError(null);
      const result = await apiLocalLogin(username.trim(), password);
      if (!result.ok) {
        setBusy(false);
        setError(loginError(result.reason));
        return;
      }

      const authenticatedUser = result.data.username?.trim() || username.trim() || DEFAULT_DASHBOARD_USERNAME;
      if (result.data.access_token) {
        saveSession(authenticatedUser, 'bearer', result.data.access_token);
      } else if (result.data.auth_mode === 'cookie') {
        saveSession(authenticatedUser, 'cookie');
      } else {
        setBusy(false);
        setError('Aether returned an unsupported authentication response.');
        return;
      }
      setBusy(false);
      onAuthenticated(authenticatedUser);
    },
    [onAuthenticated, password, username],
  );

  const submitLdap = useCallback(
    async (event: React.FormEvent) => {
      event.preventDefault();
      setLdapBusy(true);
      setLdapError(null);
      const result = await apiLdapLogin(username.trim(), ldapPassword);
      if (!result.ok) {
        setLdapBusy(false);
        setLdapError(loginError(result.reason, 'Active Directory username or password'));
        return;
      }
      const authenticatedUser = result.data.display_name?.trim() || result.data.username || username.trim();
      saveSession(authenticatedUser, 'cookie');
      setLdapBusy(false);
      onAuthenticated(authenticatedUser);
    },
    [ldapPassword, onAuthenticated, username],
  );

  const submitApiKey = useCallback(
    async (event: React.FormEvent) => {
      event.preventDefault();
      setApiKeyBusy(true);
      setApiKeyError(null);
      const result = await apiTryAuth(apiKey.trim());
      if (!result.ok) {
        setApiKeyBusy(false);
        setApiKeyError(loginError(result.reason, 'bearer API key'));
        return;
      }
      const authenticatedUser = username.trim() || DEFAULT_DASHBOARD_USERNAME;
      saveSession(authenticatedUser, 'bearer', apiKey.trim());
      setApiKeyBusy(false);
      onAuthenticated(authenticatedUser);
    },
    [apiKey, onAuthenticated, username],
  );

  const redirectToProvider = useCallback((kind: 'oidc' | 'saml') => {
    const next = `${window.location.pathname}${window.location.search}${window.location.hash}` || '/';
    const advertisedUrl = providers?.[kind]?.login_url;
    const fallback = `/api/auth/${kind}/login`;
    const separator = (advertisedUrl ?? fallback).includes('?') ? '&' : '?';
    window.location.assign(`${advertisedUrl ?? fallback}${separator}next=${encodeURIComponent(next)}`);
  }, [providers]);

  const hasEnterpriseProvider = Boolean(
    providers?.oidc?.enabled || providers?.saml?.enabled || providers?.ldap?.enabled,
  );

  return (
    <main className="login-scroll" data-testid="login-gate">
      <div className="ae-ambient" aria-hidden><i /><i /><i /></div>

      <section className="login-chapter login-hero text-center" aria-label="Aether">
        <div className="relative z-10 flex flex-col items-center">
          <div className="flex items-center gap-3">
            <div className="h-10 w-10 rounded-xl glass-fill flex items-center justify-center">
              <Hexagon className="h-5 w-5 text-primary" strokeWidth={1.6} aria-hidden />
            </div>
            <div className="text-left">
              <div className="text-lg font-semibold tracking-tight">Aether</div>
              <div className="text-[11px] uppercase tracking-[0.18em] text-muted">Universal runtime control plane</div>
            </div>
          </div>
          {health ? (
            <span
              data-tone="sky"
              className="hero-swatch mt-4"
              title={`Host: ${health.hostname}${health.environment ? ` · Environment: ${health.environment}` : ''}`}
            >
              <span className="hero-swatch-dot" aria-hidden />
              {health.environment ?? health.hostname}
            </span>
          ) : null}

          <div className="login-fade-in glass-fill mt-9 inline-flex items-center gap-2 rounded-full px-3 py-1.5 text-xs text-muted">
            <Radio className="h-3.5 w-3.5 text-primary" aria-hidden />
            One control plane. Every runtime.
          </div>
          <h1 className="login-fade-in login-fade-in-d1 mt-7 max-w-3xl text-4xl sm:text-5xl xl:text-6xl font-semibold tracking-[-0.04em] leading-[1.04]">
            Deploy anywhere
            <br />
            <span className="login-text-gradient">from one YAML spec.</span>
          </h1>
          <p className="login-fade-in login-fade-in-d2 mt-6 max-w-xl text-base sm:text-lg leading-relaxed text-muted">
            Orchestrate Podman, Kubernetes, and KubeVirt with unified lifecycle management,
            policy, and real-time intelligence.
          </p>

          <div className="login-fade-in login-fade-in-d3 mt-9 grid w-full max-w-xl gap-3 sm:grid-cols-3">
            {[
              { icon: Layers, title: 'One specification', text: 'Portable workload intent', tone: 'sky' as const },
              { icon: ShieldCheck, title: 'Policy built in', text: 'RBAC and audit trails', tone: 'violet' as const },
              { icon: Boxes, title: 'Runtime aware', text: 'Smart placement decisions', tone: 'teal' as const },
            ].map(({ icon: Icon, title, text, tone }) => (
              <div key={title} data-tone={tone} className="glass-fill rounded-xl p-4 text-left" style={{ background: 'color-mix(in srgb, var(--tone-color) 14%, var(--glass-bg))' }}>
                <Icon className="h-4 w-4" style={{ color: 'var(--tone-color)' }} aria-hidden />
                <div className="mt-3 text-sm font-medium text-foreground">{title}</div>
                <div className="mt-1 text-xs leading-relaxed text-muted">{text}</div>
              </div>
            ))}
          </div>

          <div className="login-fade-in login-fade-in-d4 mt-8 hero-swatch-row justify-center">
            {[
              { icon: Container, label: 'Podman', tone: 'sky' as const },
              { icon: Server, label: 'Kubernetes', tone: 'violet' as const },
              { icon: Boxes, label: 'KubeVirt', tone: 'teal' as const },
            ].map(({ icon: Icon, label, tone }) => (
              <span key={label} data-tone={tone} className="hero-swatch">
                <Icon className="h-3 w-3" style={{ color: 'var(--tone-color)' }} aria-hidden />
                {label}
              </span>
            ))}
          </div>

          <a href="#login-sign-in-chapter" className="login-scroll-cue mt-14">
            Continue to sign in
            <ChevronDown className="h-4 w-4" aria-hidden />
          </a>

          <a
            href={ZYVOR_URL}
            target="_blank"
            rel="noopener noreferrer"
            className="mt-8 inline-flex items-center gap-2 text-xs font-medium"
            style={{ color: ZYVOR_ORANGE }}
          >
            <img src="/zyvor-logo.svg" alt="" width={16} height={16} className="rounded-[4px]" />
            Built by ZyvorAI Labs
          </a>
        </div>
      </section>

      <section id="login-sign-in-chapter" className="login-chapter login-panel" aria-label="Sign in">
        <div className="w-full max-w-md">
          <div className="mb-8 text-center">
            <p className="text-xs font-semibold uppercase tracking-[0.16em] text-primary">Aether console</p>
            <h2 className="mt-3 text-3xl font-semibold tracking-tight text-foreground">Welcome back</h2>
            <p className="mt-2 text-sm leading-relaxed text-muted">
              Sign in to manage workloads across every connected runtime.
            </p>
          </div>

          <div className="login-glass p-5 sm:p-6">
            {notice ? (
              <div className="mb-4 rounded-xl border border-warning bg-warning px-3.5 py-3 text-sm text-warning" role="alert">
                {notice}
              </div>
            ) : null}
            {error ? (
              <div className="mb-4 rounded-xl border border-danger bg-danger px-3.5 py-3 text-sm text-danger" role="alert">
                {error}
              </div>
            ) : null}

            <form onSubmit={submitLocal} className="space-y-4" data-testid="local-login-form">
              <div>
                <label htmlFor="login-username" className="mb-1.5 block text-sm font-medium text-foreground">Username</label>
                <div className="relative">
                  <User className="absolute left-3.5 top-1/2 h-4 w-4 -translate-y-1/2 text-muted" aria-hidden />
                  <input
                    id="login-username"
                    className="login-input"
                    value={username}
                    onChange={(event) => {
                      setUsername(event.target.value);
                      setError(null);
                    }}
                    autoComplete="username"
                    required
                  />
                </div>
              </div>

              <div>
                <label htmlFor="login-password" className="mb-1.5 block text-sm font-medium text-foreground">Password</label>
                <div className="relative">
                  <LockKeyhole className="absolute left-3.5 top-1/2 h-4 w-4 -translate-y-1/2 text-muted" aria-hidden />
                  <input
                    id="login-password"
                    className="login-input pr-11"
                    type={showPassword ? 'text' : 'password'}
                    value={password}
                    onChange={(event) => {
                      setPassword(event.target.value);
                      setError(null);
                    }}
                    autoComplete="current-password"
                    required
                  />
                  <button
                    type="button"
                    onClick={() => setShowPassword((visible) => !visible)}
                    className="absolute right-3.5 top-1/2 -translate-y-1/2 text-muted transition-colors hover:text-foreground"
                    aria-label={showPassword ? 'Hide password' : 'Show password'}
                  >
                    {showPassword ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                  </button>
                </div>
              </div>

              <div className="flex items-center justify-between gap-3 text-xs">
                <span className="text-muted">Demo: <strong className="font-mono text-foreground">admin / Admin@321</strong></span>
                {providers?.local?.note ? <span className="sr-only">{providers.local.note}</span> : null}
              </div>

              <button className="login-btn-primary" type="submit" disabled={busy || !username.trim() || !password}>
                {busy ? <Loader2 className="h-4 w-4 animate-spin" aria-hidden /> : <LogIn className="h-4 w-4" aria-hidden />}
                {busy ? 'Signing in…' : 'Sign in'}
                {!busy ? <ArrowRight className="h-4 w-4" aria-hidden /> : null}
              </button>
            </form>
          </div>

          {providersLoading ? (
            <div className="mt-5 h-10 animate-pulse rounded-xl bg-surface" aria-hidden />
          ) : hasEnterpriseProvider ? (
            <div className="mt-5">
              <div className="mb-4 flex items-center gap-3 text-[11px] uppercase tracking-[0.14em] text-muted">
                <span className="h-px flex-1 bg-border" />
                Organization sign-in
                <span className="h-px flex-1 bg-border" />
              </div>
              <div className="grid gap-2">
                {providers?.oidc?.enabled ? (
                  <button type="button" onClick={() => redirectToProvider('oidc')} className="flex w-full items-center justify-center gap-2 rounded-xl border border-border px-4 py-2.5 text-sm font-medium text-foreground transition-colors hover:border-border hover:bg-surface" data-testid="login-oidc-button">
                    <LogIn className="h-4 w-4" aria-hidden /> Sign in with OIDC
                  </button>
                ) : null}
                {providers?.saml?.enabled ? (
                  <button type="button" onClick={() => redirectToProvider('saml')} className="flex w-full items-center justify-center gap-2 rounded-xl border border-border px-4 py-2.5 text-sm font-medium text-foreground transition-colors hover:border-border hover:bg-surface" data-testid="login-saml-button">
                    <LogIn className="h-4 w-4" aria-hidden /> Sign in with SAML
                  </button>
                ) : null}
                {providers?.ldap?.enabled ? (
                  <button type="button" onClick={() => setShowLdap((open) => !open)} className="flex w-full items-center justify-center gap-2 rounded-xl border border-border px-4 py-2.5 text-sm font-medium text-foreground transition-colors hover:border-border hover:bg-surface" aria-expanded={showLdap}>
                    <Server className="h-4 w-4" aria-hidden /> Active Directory
                  </button>
                ) : null}
              </div>

              {showLdap && providers?.ldap?.enabled ? (
                <form onSubmit={submitLdap} className="mt-3 rounded-xl border border-border bg-surface p-4 space-y-3" data-testid="ldap-login-form">
                  <p className="text-xs text-muted">
                    {providers.ldap.domain ? `Use your ${providers.ldap.domain} account.` : 'Use your directory account.'}
                  </p>
                  <input type="password" value={ldapPassword} onChange={(event) => setLdapPassword(event.target.value)} className="login-input !pl-3.5" placeholder="Directory password" autoComplete="current-password" required />
                  {ldapError ? <p className="text-xs text-danger" role="alert">{ldapError}</p> : null}
                  <button type="submit" className="login-btn-primary" disabled={ldapBusy || !username.trim() || !ldapPassword}>
                    {ldapBusy ? <Loader2 className="h-4 w-4 animate-spin" /> : null}
                    {ldapBusy ? 'Signing in…' : 'Continue with Active Directory'}
                  </button>
                </form>
              ) : null}
            </div>
          ) : null}

          <div className="mt-5 text-center">
            <button type="button" onClick={() => setShowApiKey((open) => !open)} className="inline-flex items-center gap-1.5 text-xs font-medium text-muted transition-colors hover:text-foreground" aria-expanded={showApiKey}>
              <KeyRound className="h-3.5 w-3.5" aria-hidden />
              Advanced: use bearer API key
            </button>
            {showApiKey ? (
              <form onSubmit={submitApiKey} className="mt-3 rounded-xl border border-border p-4 text-left">
                <label htmlFor="login-api-key" className="mb-1.5 block text-xs font-medium text-muted">Bearer API key</label>
                <div className="relative">
                  <KeyRound className="absolute left-3.5 top-1/2 h-4 w-4 -translate-y-1/2 text-muted" aria-hidden />
                  <input id="login-api-key" type={showApiKeyValue ? 'text' : 'password'} value={apiKey} onChange={(event) => setApiKey(event.target.value)} className="login-input pr-11 font-mono" autoComplete="off" />
                  <button type="button" onClick={() => setShowApiKeyValue((visible) => !visible)} className="absolute right-3.5 top-1/2 -translate-y-1/2 text-muted" aria-label={showApiKeyValue ? 'Hide API key' : 'Show API key'}>
                    {showApiKeyValue ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                  </button>
                </div>
                {apiKeyError ? <p className="mt-2 text-xs text-danger" role="alert">{apiKeyError}</p> : null}
                <button type="submit" className="mt-3 w-full rounded-xl border border-border px-4 py-2.5 text-sm font-medium text-foreground hover:bg-surface disabled:opacity-50" disabled={apiKeyBusy}>
                  {apiKeyBusy ? 'Verifying…' : 'Continue with API key'}
                </button>
              </form>
            ) : null}
          </div>
        </div>

        <ZyvorFooter className="mt-8" />
      </section>
    </main>
  );
}
