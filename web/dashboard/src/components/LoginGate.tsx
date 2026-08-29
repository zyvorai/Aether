// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useState } from 'react';
import {
  ArrowRight,
  Boxes,
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
  apiLdapLogin,
  apiLocalLogin,
  apiTryAuth,
  DEFAULT_DASHBOARD_USERNAME,
  type AuthProvidersPayload,
} from '../utils/api';
import { ZyvorFooter } from './ZyvorBrand';

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
    <main className="min-h-screen grid lg:grid-cols-[minmax(0,1.08fr)_minmax(440px,0.92fr)]" data-testid="login-gate">
      <section className="login-hero min-h-[320px] lg:min-h-screen px-8 py-10 sm:px-12 lg:px-16 xl:px-24 flex flex-col text-white">
        <div className="login-hero-mesh" aria-hidden />
        <div className="absolute -left-32 top-1/4 h-80 w-80 rounded-full bg-primary/10 blur-3xl" aria-hidden />
        <div className="relative z-10 flex items-center gap-3">
          <div className="h-10 w-10 rounded-xl border border-white/15 bg-white/10 flex items-center justify-center shadow-lg">
            <Hexagon className="h-5 w-5 text-sky-300" strokeWidth={1.6} aria-hidden />
          </div>
          <div>
            <div className="text-lg font-semibold tracking-tight">Aether</div>
            <div className="text-[11px] uppercase tracking-[0.18em] text-white/50">Universal runtime control plane</div>
          </div>
        </div>

        <div className="relative z-10 my-auto max-w-2xl py-16 lg:py-20">
          <div className="login-fade-in inline-flex items-center gap-2 rounded-full border border-white/10 bg-white/[0.06] px-3 py-1.5 text-xs text-white/70">
            <Radio className="h-3.5 w-3.5 text-sky-300" aria-hidden />
            One control plane. Every runtime.
          </div>
          <h1 className="login-fade-in login-fade-in-d1 mt-7 text-4xl sm:text-5xl xl:text-6xl font-semibold tracking-[-0.04em] leading-[1.04]">
            Deploy anywhere
            <br />
            <span className="login-text-gradient">from one YAML spec.</span>
          </h1>
          <p className="login-fade-in login-fade-in-d2 mt-6 max-w-xl text-base sm:text-lg leading-relaxed text-white/60">
            Orchestrate Podman, Kubernetes, KubeVirt, and Metal3 with unified lifecycle management,
            policy, and real-time intelligence.
          </p>

          <div className="login-fade-in login-fade-in-d3 mt-9 grid max-w-xl gap-3 sm:grid-cols-3">
            {[
              { icon: Layers, title: 'One specification', text: 'Portable workload intent' },
              { icon: ShieldCheck, title: 'Policy built in', text: 'RBAC and audit trails' },
              { icon: Boxes, title: 'Runtime aware', text: 'Smart placement decisions' },
            ].map(({ icon: Icon, title, text }) => (
              <div key={title} className="rounded-xl border border-white/10 bg-white/[0.045] p-4 backdrop-blur-sm">
                <Icon className="h-4 w-4 text-sky-300" aria-hidden />
                <div className="mt-3 text-sm font-medium text-white/90">{title}</div>
                <div className="mt-1 text-xs leading-relaxed text-white/45">{text}</div>
              </div>
            ))}
          </div>

          <div className="login-fade-in login-fade-in-d4 mt-8 flex flex-wrap gap-2 text-xs text-white/55">
            {[
              { icon: Container, label: 'Podman' },
              { icon: Server, label: 'Kubernetes' },
              { icon: Boxes, label: 'KubeVirt' },
              { icon: Server, label: 'Metal3' },
            ].map(({ icon: Icon, label }) => (
              <span key={label} className="inline-flex items-center gap-1.5 rounded-full border border-white/10 px-3 py-1.5">
                <Icon className="h-3 w-3" aria-hidden />
                {label}
              </span>
            ))}
          </div>
        </div>

        <p className="relative z-10 text-xs text-white/35">Built by ZyvorAI Labs</p>
      </section>

      <section className="login-panel min-h-screen px-5 py-10 sm:px-10 lg:px-14 xl:px-20 flex flex-col">
        <div className="m-auto w-full max-w-md">
          <div className="mb-8">
            <p className="text-xs font-semibold uppercase tracking-[0.16em] text-primary">Aether console</p>
            <h2 className="mt-3 text-3xl font-semibold tracking-tight text-slate-950">Welcome back</h2>
            <p className="mt-2 text-sm leading-relaxed text-slate-500">
              Sign in to manage workloads across every connected runtime.
            </p>
          </div>

          <div className="login-glass p-5 sm:p-6">
            {notice ? (
              <div className="mb-4 rounded-xl border border-amber-200 bg-amber-50 px-3.5 py-3 text-sm text-amber-900" role="alert">
                {notice}
              </div>
            ) : null}
            {error ? (
              <div className="mb-4 rounded-xl border border-red-200 bg-red-50 px-3.5 py-3 text-sm text-red-700" role="alert">
                {error}
              </div>
            ) : null}

            <form onSubmit={submitLocal} className="space-y-4" data-testid="local-login-form">
              <div>
                <label htmlFor="login-username" className="mb-1.5 block text-sm font-medium text-slate-700">Username</label>
                <div className="relative">
                  <User className="absolute left-3.5 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-400" aria-hidden />
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
                <label htmlFor="login-password" className="mb-1.5 block text-sm font-medium text-slate-700">Password</label>
                <div className="relative">
                  <LockKeyhole className="absolute left-3.5 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-400" aria-hidden />
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
                    className="absolute right-3.5 top-1/2 -translate-y-1/2 text-slate-400 transition-colors hover:text-slate-700"
                    aria-label={showPassword ? 'Hide password' : 'Show password'}
                  >
                    {showPassword ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                  </button>
                </div>
              </div>

              <div className="flex items-center justify-between gap-3 text-xs">
                <span className="text-slate-500">Demo: <strong className="font-mono text-slate-700">admin / Admin@321</strong></span>
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
            <div className="mt-5 h-10 animate-pulse rounded-xl bg-slate-100" aria-hidden />
          ) : hasEnterpriseProvider ? (
            <div className="mt-5">
              <div className="mb-4 flex items-center gap-3 text-[11px] uppercase tracking-[0.14em] text-slate-400">
                <span className="h-px flex-1 bg-slate-200" />
                Organization sign-in
                <span className="h-px flex-1 bg-slate-200" />
              </div>
              <div className="grid gap-2">
                {providers?.oidc?.enabled ? (
                  <button type="button" onClick={() => redirectToProvider('oidc')} className="flex w-full items-center justify-center gap-2 rounded-xl border border-slate-200 px-4 py-2.5 text-sm font-medium text-slate-700 transition-colors hover:border-slate-300 hover:bg-slate-50" data-testid="login-oidc-button">
                    <LogIn className="h-4 w-4" aria-hidden /> Sign in with OIDC
                  </button>
                ) : null}
                {providers?.saml?.enabled ? (
                  <button type="button" onClick={() => redirectToProvider('saml')} className="flex w-full items-center justify-center gap-2 rounded-xl border border-slate-200 px-4 py-2.5 text-sm font-medium text-slate-700 transition-colors hover:border-slate-300 hover:bg-slate-50" data-testid="login-saml-button">
                    <LogIn className="h-4 w-4" aria-hidden /> Sign in with SAML
                  </button>
                ) : null}
                {providers?.ldap?.enabled ? (
                  <button type="button" onClick={() => setShowLdap((open) => !open)} className="flex w-full items-center justify-center gap-2 rounded-xl border border-slate-200 px-4 py-2.5 text-sm font-medium text-slate-700 transition-colors hover:border-slate-300 hover:bg-slate-50" aria-expanded={showLdap}>
                    <Server className="h-4 w-4" aria-hidden /> Active Directory
                  </button>
                ) : null}
              </div>

              {showLdap && providers?.ldap?.enabled ? (
                <form onSubmit={submitLdap} className="mt-3 rounded-xl border border-slate-200 bg-slate-50 p-4 space-y-3" data-testid="ldap-login-form">
                  <p className="text-xs text-slate-500">
                    {providers.ldap.domain ? `Use your ${providers.ldap.domain} account.` : 'Use your directory account.'}
                  </p>
                  <input type="password" value={ldapPassword} onChange={(event) => setLdapPassword(event.target.value)} className="login-input !pl-3.5" placeholder="Directory password" autoComplete="current-password" required />
                  {ldapError ? <p className="text-xs text-red-600" role="alert">{ldapError}</p> : null}
                  <button type="submit" className="login-btn-primary" disabled={ldapBusy || !username.trim() || !ldapPassword}>
                    {ldapBusy ? <Loader2 className="h-4 w-4 animate-spin" /> : null}
                    {ldapBusy ? 'Signing in…' : 'Continue with Active Directory'}
                  </button>
                </form>
              ) : null}
            </div>
          ) : null}

          <div className="mt-5 text-center">
            <button type="button" onClick={() => setShowApiKey((open) => !open)} className="inline-flex items-center gap-1.5 text-xs font-medium text-slate-500 transition-colors hover:text-slate-800" aria-expanded={showApiKey}>
              <KeyRound className="h-3.5 w-3.5" aria-hidden />
              Advanced: use bearer API key
            </button>
            {showApiKey ? (
              <form onSubmit={submitApiKey} className="mt-3 rounded-xl border border-slate-200 p-4 text-left">
                <label htmlFor="login-api-key" className="mb-1.5 block text-xs font-medium text-slate-600">Bearer API key</label>
                <div className="relative">
                  <KeyRound className="absolute left-3.5 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-400" aria-hidden />
                  <input id="login-api-key" type={showApiKeyValue ? 'text' : 'password'} value={apiKey} onChange={(event) => setApiKey(event.target.value)} className="login-input pr-11 font-mono" autoComplete="off" />
                  <button type="button" onClick={() => setShowApiKeyValue((visible) => !visible)} className="absolute right-3.5 top-1/2 -translate-y-1/2 text-slate-400" aria-label={showApiKeyValue ? 'Hide API key' : 'Show API key'}>
                    {showApiKeyValue ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                  </button>
                </div>
                {apiKeyError ? <p className="mt-2 text-xs text-red-600" role="alert">{apiKeyError}</p> : null}
                <button type="submit" className="mt-3 w-full rounded-xl border border-slate-300 px-4 py-2.5 text-sm font-medium text-slate-700 hover:bg-slate-50 disabled:opacity-50" disabled={apiKeyBusy}>
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
