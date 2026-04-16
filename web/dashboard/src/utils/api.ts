import type { ApiResponse } from '../types/api';

const BASE = '/api';

function authHeaders(extra?: Record<string, string>): Record<string, string> {
  const headers: Record<string, string> = { ...extra };
  const stored = sessionStorage.getItem('aether_auth');
  if (stored) {
    try {
      const parsed = JSON.parse(stored);
      if (parsed.token) headers['Authorization'] = `Bearer ${parsed.token}`;
    } catch { /* ignore */ }
  }
  return headers;
}

export async function apiFetch<T>(path: string): Promise<T | null> {
  try {
    const res = await fetch(BASE + path, { headers: authHeaders() });
    const json: ApiResponse<T> = await res.json();
    return json.success ? json.data : null;
  } catch {
    return null;
  }
}

export async function apiPost<T>(path: string, body?: unknown): Promise<ApiResponse<T>> {
  try {
    const res = await fetch(BASE + path, {
      method: 'POST',
      headers: authHeaders({ 'Content-Type': 'application/json' }),
      body: JSON.stringify(body ?? {}),
    });
    return await res.json();
  } catch (e) {
    return { success: false, data: null, error: String(e) };
  }
}

export async function apiDelete(path: string): Promise<ApiResponse<string>> {
  try {
    const res = await fetch(BASE + path, {
      method: 'DELETE',
      headers: authHeaders(),
    });
    return await res.json();
  } catch (e) {
    return { success: false, data: null, error: String(e) };
  }
}

export async function apiPut<T>(path: string, body?: unknown): Promise<ApiResponse<T>> {
  try {
    const res = await fetch(BASE + path, {
      method: 'PUT',
      headers: authHeaders({ 'Content-Type': 'application/json' }),
      body: body ? JSON.stringify(body) : undefined,
    });
    return await res.json();
  } catch (e) {
    return { success: false, data: null, error: String(e) };
  }
}

export async function apiText(path: string): Promise<string> {
  try {
    const res = await fetch(BASE + path, { headers: authHeaders() });
    return await res.text();
  } catch {
    return 'Failed to fetch';
  }
}
