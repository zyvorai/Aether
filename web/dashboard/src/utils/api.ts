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

export async function apiFetch<T>(path: string): Promise<T | null> {
  try {
    const res = await fetch(BASE + path, withCreds({ headers: authHeaders() }));
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
    return await res.json();
  } catch (e) {
    return { success: false, data: null, error: String(e) };
  }
}

export async function apiText(path: string): Promise<string> {
  try {
    const res = await fetch(BASE + path, withCreds({ headers: authHeaders() }));
    return await res.text();
  } catch {
    return 'Failed to fetch';
  }
}

export async function apiTextSettled(path: string): Promise<FetchSettledResult<string>> {
  try {
    const res = await fetch(BASE + path, withCreds({ headers: authHeaders() }));
    if (!res.ok) return { ok: false, error: 'api' };
    return { ok: true, data: await res.text() };
  } catch {
    return { ok: false, error: 'network' };
  }
}
