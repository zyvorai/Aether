// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import type { ApiResponse, AuthStatus } from '../types/api';
import { getActiveTenantId } from './tenantContext';

const BASE = '/api';
export const DEFAULT_DASHBOARD_USERNAME = 'admin';

/** Same-origin OIDC session cookie (`aether_session`) is sent only when credentials are included. */
function withCreds(init?: RequestInit): RequestInit {
  return { credentials: 'include', ...init };
}

/**
 * Fired when a data request comes back 401: the current session's credential (bearer
 * token, cookie, or the open local-dev bypass) is no longer valid — e.g. the server just
 * required auth for the first time because an RBAC key was created. App.tsx listens for
 * this to drop back to the login screen instead of leaving every page stuck on a generic
 * "could not load" error with no way to recover.
 */
export const UNAUTHORIZED_EVENT = 'aether-unauthorized';

/**
 * Suppresses the unauthorized-redirect while a one-time secret (e.g. a freshly created RBAC
 * key's plaintext value) is on screen. Several independent background pollers run at the app
 * root regardless of which page is open (ServerCapabilitiesContext's 12s /server refresh, the
 * SSE stream, etc.) — any one of them can 401 within seconds of the very action that ended the
 * local-dev auth bypass, which would otherwise boot the whole app back to the login screen and
 * take the modal showing that secret down with it before the user can copy it. A counter (not
 * a boolean) so nested/concurrent suppressions can't accidentally re-enable each other early.
 */
let unauthorizedSuppressionCount = 0;

export function suppressUnauthorizedRedirect(): void {
  unauthorizedSuppressionCount += 1;
}

export function allowUnauthorizedRedirect(): void {
  unauthorizedSuppressionCount = Math.max(0, unauthorizedSuppressionCount - 1);
}

function notifyIfUnauthorized(res: Response): void {
  if (res.status === 401 && unauthorizedSuppressionCount === 0) {
    window.dispatchEvent(new CustomEvent(UNAUTHORIZED_EVENT));
  }
}

/** Build-time optional dev default (VITE_AETHER_DEFAULT_API_KEY). Never set in production images. */
export function getDevBootstrapApiKey(): string | undefined {
  const v = import.meta.env.VITE_AETHER_DEFAULT_API_KEY;
  return typeof v === 'string' && v.trim() !== '' ? v.trim() : undefined;
}

export function getAuthToken(): string | null {
  const stored = sessionStorage.getItem('aether_auth');
  if (stored) {
    try {
      const parsed = JSON.parse(stored) as { token?: unknown; authMode?: string };
      if (parsed.authMode === 'cookie') {
        return null;
      }
      if (parsed.token !== undefined && parsed.token !== null) {
        const t = String(parsed.token).trim();
        return t === '' ? null : t;
      }
    } catch {
      /* ignore */
    }
  }
  return null;
}

export type DashboardAuthMode = 'bearer' | 'cookie' | 'dev';

export function getDashboardAuthMode(): DashboardAuthMode | null {
  const stored = sessionStorage.getItem('aether_auth');
  if (!stored) return null;
  try {
    const parsed = JSON.parse(stored) as { authMode?: DashboardAuthMode };
    return parsed.authMode ?? 'bearer';
  } catch {
    return null;
  }
}

/** After OIDC redirect, `/api/auth/me` succeeds via session cookie without a bearer token. */
export async function apiTryCookieSession(): Promise<{ username: string } | null> {
  try {
    const res = await fetch(`${BASE}/auth/me`, withCreds());
    const json: ApiResponse<AuthStatus> = await res.json();
    if (json.success && json.data?.authenticated) {
      return { username: json.data.username };
    }
    return null;
  } catch {
    return null;
  }
}

export interface AuthProvidersOidc {
  enabled: boolean;
  issuer?: string | null;
  authorization_code_pkce?: boolean;
  login_url?: string;
  logout_url?: string;
  note?: string;
}

export interface AuthProvidersPayload {
  methods: string[];
  oidc: AuthProvidersOidc;
  saml?: {
    enabled: boolean;
    login_url?: string;
  };
  ldap?: {
    enabled: boolean;
    domain?: string | null;
    login_url?: string;
    logout_url?: string;
    note?: string;
  };
}

export interface AuthSettingsSession {
  cookie_name: string;
  session_secret_configured: boolean;
  redis_configured: boolean;
  jwt_session_hours: number;
  note?: string;
}

export interface AuthSettingsOidc {
  enabled: boolean;
  issuer?: string | null;
  client_id?: string | null;
  redirect_uri?: string | null;
  login_path?: string;
  callback_path?: string;
  logout_path?: string;
  client_secret_configured?: boolean;
  role_mapping_configured?: boolean;
  groups_claim?: string;
  default_role?: string;
  flow?: string[];
  discovery?: string | null;
  note?: string;
}

export interface AuthSettingsSaml {
  enabled: boolean;
  sp_entity_id?: string | null;
  acs_url?: string | null;
  idp_sso_url?: string | null;
  idp_entity_id?: string | null;
  idp_cert_configured?: boolean;
  metadata_path?: string;
  login_path?: string;
  acs_path?: string;
  logout_path?: string;
  role_mapping_configured?: boolean;
  default_role?: string;
  note?: string;
}

export interface AuthSettingsLdap {
  enabled: boolean;
  url?: string | null;
  base_dn?: string | null;
  domain?: string | null;
  bind_dn_configured?: boolean;
  role_mapping_configured?: boolean;
  default_role?: string;
  login_path?: string;
  logout_path?: string;
  note?: string;
}

export interface AuthSettingsPayload {
  session: AuthSettingsSession;
  oidc: AuthSettingsOidc;
  saml: AuthSettingsSaml;
  ldap: AuthSettingsLdap;
}

export async function apiFetchAuthSettings(): Promise<AuthSettingsPayload | null> {
  try {
    const res = await fetch(`${BASE}/auth/settings`, withCreds({ headers: authHeaders() }));
    const json: ApiResponse<AuthSettingsPayload> = await res.json();
    return json.success && json.data ? json.data : null;
  } catch {
    return null;
  }
}

export async function apiFetchAuthProviders(): Promise<AuthProvidersPayload | null> {
  try {
    const res = await fetch(`${BASE}/auth/providers`, withCreds());
    const json: ApiResponse<AuthProvidersPayload> = await res.json();
    return json.success && json.data ? json.data : null;
  } catch {
    return null;
  }
}

function authHeaders(extra?: Record<string, string>): Record<string, string> {
  const headers: Record<string, string> = { ...extra };
  const token = getAuthToken();
  if (token !== null && token.trim() !== '') {
    headers.Authorization = `Bearer ${token}`;
  }
  const tenant = getActiveTenantId();
  if (tenant) {
    headers['X-Aether-Tenant'] = tenant;
  }
  return headers;
}

export interface ServerSafety {
  mutation_confirm_required: boolean;
  mutation_confirm_header: string;
  mutation_confirm_values?: string[];
}

let cachedServerSafety: ServerSafety | null | undefined;

/** Called by ServerCapabilitiesProvider after loading `/api/server`. */
export function primeServerSafety(safety: ServerSafety | null): void {
  cachedServerSafety = safety;
}

export async function getServerSafety(): Promise<ServerSafety | null> {
  if (cachedServerSafety !== undefined) {
    return cachedServerSafety;
  }
  try {
    const res = await fetch(`${BASE}/server`, withCreds({ headers: authHeaders() }));
    const json: ApiResponse<{ safety?: ServerSafety }> = await res.json();
    const safety = json.success && json.data?.safety ? json.data.safety : null;
    cachedServerSafety = safety;
    return safety;
  } catch {
    cachedServerSafety = null;
    return null;
  }
}

/** User confirmation when the API has `AETHER_REQUIRE_MUTATION_CONFIRM` enabled. */
export async function confirmDestructive(label: string): Promise<boolean> {
  const safety = await getServerSafety();
  if (!safety?.mutation_confirm_required) {
    return true;
  }
  return window.confirm(
    `${label}\n\nThe server requires explicit confirmation for this action. Continue?`,
  );
}

/** SSE cannot send Authorization headers; bearer sessions use `?token=` (cookies work same-origin). */
export function apiEventStreamUrl(): string {
  const url = new URL(`${BASE}/events/stream`, window.location.origin);
  const token = getAuthToken();
  if (token) {
    url.searchParams.set('token', token);
  }
  return url.pathname + url.search;
}

export function apiWebSocketUrl(path: string): string {
  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
  const url = new URL(`/api${path}`, `${protocol}//${window.location.host}`);
  const token = getAuthToken();
  if (token !== null && token.trim() !== '') {
    url.searchParams.set('token', token);
  }
  return url.toString();
}

/** Verify bearer token (or open dev) against GET /api/auth/me. */
export type AuthTryResult =
  | { ok: true }
  | { ok: false; reason: 'network' | 'unauthorized' | 'rejected' };

export async function apiTryAuth(token: string): Promise<AuthTryResult> {
  const headers: Record<string, string> = {};
  const t = token.trim();
  if (t) headers.Authorization = `Bearer ${t}`;
  try {
    const res = await fetch(`${BASE}/auth/me`, withCreds({ headers }));
    if (res.status === 401 || res.status === 403) {
      return { ok: false, reason: 'unauthorized' };
    }
    const json: ApiResponse<unknown> = await res.json();
    return json.success === true ? { ok: true } : { ok: false, reason: 'rejected' };
  } catch {
    return { ok: false, reason: 'network' };
  }
}

export interface LdapLoginResult {
  username: string;
  role: string;
  display_name?: string | null;
}

export async function apiLdapLogin(
  username: string,
  password: string,
): Promise<{ ok: true; data: LdapLoginResult } | { ok: false; reason: 'network' | 'unauthorized' | 'rejected' }> {
  try {
    const res = await fetch(`${BASE}/auth/ldap/login`, withCreds({
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ username, password }),
    }));
    const json: ApiResponse<LdapLoginResult> = await res.json();
    if (res.status === 401 || res.status === 403) {
      return { ok: false, reason: 'unauthorized' };
    }
    if (json.success && json.data) {
      return { ok: true, data: json.data };
    }
    return { ok: false, reason: 'rejected' };
  } catch {
    return { ok: false, reason: 'network' };
  }
}

export async function apiFetch<T>(path: string): Promise<T | null> {
  try {
    const res = await fetch(BASE + path, withCreds({ headers: authHeaders() }));
    notifyIfUnauthorized(res);
    const json: ApiResponse<T> = await res.json();
    return json.success ? json.data : null;
  } catch {
    return null;
  }
}

export type FetchSettledResult<T> =
  | { ok: true; data: T }
  | { ok: false; error: 'network' | 'api' };

/** Like apiFetch but distinguishes network vs API failures for page-level error UI. */
export async function apiFetchSettled<T>(path: string): Promise<FetchSettledResult<T>> {
  try {
    const res = await fetch(BASE + path, withCreds({ headers: authHeaders() }));
    notifyIfUnauthorized(res);
    const json: ApiResponse<T> = await res.json();
    if (json.success && json.data !== undefined && json.data !== null) {
      return { ok: true, data: json.data };
    }
    return { ok: false, error: 'api' };
  } catch {
    return { ok: false, error: 'network' };
  }
}

export async function apiPost<T>(path: string, body?: unknown): Promise<ApiResponse<T>> {
  try {
    const res = await fetch(
      BASE + path,
      withCreds({
        method: 'POST',
        headers: authHeaders({ 'Content-Type': 'application/json' }),
        body: JSON.stringify(body ?? {}),
      }),
    );
    notifyIfUnauthorized(res);
    return await res.json();
  } catch (e) {
    return { success: false, data: null, error: String(e) };
  }
}

export async function apiDelete(
  path: string,
  opts?: { label?: string },
): Promise<ApiResponse<string>> {
  const label = opts?.label ?? `Delete ${path}`;
  if (!(await confirmDestructive(label))) {
    return { success: false, data: null, error: 'cancelled' };
  }
  try {
    const headers = authHeaders();
    const safety = await getServerSafety();
    if (safety?.mutation_confirm_required) {
      headers['X-Aether-Confirm'] = '1';
    }
    const res = await fetch(BASE + path, withCreds({ method: 'DELETE', headers }));
    notifyIfUnauthorized(res);
    return await res.json();
  } catch (e) {
    return { success: false, data: null, error: String(e) };
  }
}

/** DELETE with JSON body (e.g. dependency edge removal). */
export async function apiDeleteJson<T>(path: string, body: unknown): Promise<ApiResponse<T>> {
  try {
    const res = await fetch(
      BASE + path,
      withCreds({
        method: 'DELETE',
        headers: authHeaders({ 'Content-Type': 'application/json' }),
        body: JSON.stringify(body),
      }),
    );
    notifyIfUnauthorized(res);
    return await res.json();
  } catch (e) {
    return { success: false, data: null, error: String(e) };
  }
}

export async function apiPut<T>(path: string, body?: unknown): Promise<ApiResponse<T>> {
  try {
    const res = await fetch(
      BASE + path,
      withCreds({
        method: 'PUT',
        headers: authHeaders({ 'Content-Type': 'application/json' }),
        body: body ? JSON.stringify(body) : undefined,
      }),
    );
    notifyIfUnauthorized(res);
    return await res.json();
  } catch (e) {
    return { success: false, data: null, error: String(e) };
  }
}

export async function apiPostRaw<T>(path: string, body: string, contentType = 'text/plain'): Promise<ApiResponse<T>> {
  try {
    const res = await fetch(
      BASE + path,
      withCreds({
        method: 'POST',
        headers: authHeaders({ 'Content-Type': contentType }),
        body,
      }),
    );
    notifyIfUnauthorized(res);
    return await res.json();
  } catch (e) {
    return { success: false, data: null, error: String(e) };
  }
}

export async function apiText(path: string): Promise<string> {
  try {
    const res = await fetch(BASE + path, withCreds({ headers: authHeaders() }));
    notifyIfUnauthorized(res);
    return await res.text();
  } catch {
    return 'Failed to fetch';
  }
}

export async function apiTextSettled(path: string): Promise<FetchSettledResult<string>> {
  try {
    const res = await fetch(BASE + path, withCreds({ headers: authHeaders() }));
    notifyIfUnauthorized(res);
    if (!res.ok) return { ok: false, error: 'api' };
    return { ok: true, data: await res.text() };
  } catch {
    return { ok: false, error: 'network' };
  }
}
