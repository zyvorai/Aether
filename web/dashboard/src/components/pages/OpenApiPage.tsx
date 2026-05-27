// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useMemo, useState } from 'react';
import { BookOpen, Copy } from 'lucide-react';
import PageToolbar from '../PageToolbar';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';
import CodeBlock from '../CodeBlock';

interface OpenApiDoc {
  openapi?: string;
  info?: { title?: string; version?: string; description?: string };
  paths?: Record<string, Record<string, { summary?: string; description?: string }>>;
}

export default function OpenApiPage() {
  const [doc, setDoc] = useState<OpenApiDoc | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadFailed, setLoadFailed] = useState(false);
  const [search, setSearch] = useState('');
  const [methodFilter, setMethodFilter] = useState('all');
  const [copiedPath, setCopiedPath] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setLoadFailed(false);
    try {
      const res = await fetch('/api/openapi.json', { credentials: 'include' });
      if (!res.ok) {
        setLoadFailed(true);
        setDoc(null);
        setLoading(false);
        return;
      }
      const json = (await res.json()) as OpenApiDoc | { success?: boolean; data?: OpenApiDoc };
      const data =
        json && typeof json === 'object' && 'success' in json && json.success && json.data
          ? json.data
          : (json as OpenApiDoc);
      if (data?.paths) {
        setDoc(data);
      } else {
        setLoadFailed(true);
        setDoc(null);
      }
    } catch {
      setLoadFailed(true);
      setDoc(null);
    }
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const paths = useMemo(() => {
    if (!doc?.paths) return [];
    const q = search.trim().toLowerCase();
    return Object.entries(doc.paths).flatMap(([path, methods]) =>
      Object.entries(methods).map(([method, meta]) => ({
        path,
        method: method.toUpperCase(),
        summary: meta.summary ?? meta.description ?? '',
      })),
    ).filter((row) => {
      if (methodFilter !== 'all' && row.method !== methodFilter) return false;
      if (!q) return true;
      return (
        row.path.toLowerCase().includes(q)
        || row.method.toLowerCase().includes(q)
        || row.summary.toLowerCase().includes(q)
      );
    });
  }, [doc, search, methodFilter]);

  async function copyPath(path: string) {
    try {
      await navigator.clipboard.writeText(path);
      setCopiedPath(path);
      setTimeout(() => setCopiedPath(null), 2000);
    } catch {
      /* ignore */
    }
  }

  function downloadOpenApiJson() {
    if (!doc) return;
    const blob = new Blob([JSON.stringify(doc, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'aether-openapi.json';
    a.click();
    URL.revokeObjectURL(url);
  }

  if (loading && !doc && !loadFailed) {
    return <PageLoading rows={6} />;
  }

  if (loadFailed) {
    return <PageLoadError title="OpenAPI document unavailable" onRetry={() => void load()} />;
  }

  return (
    <div>
      <PageToolbar
        search={search}
        onSearchChange={setSearch}
        searchPlaceholder="Filter routes…"
        onRefresh={() => void load()}
        refreshing={loading}
        filters={
          <select
            value={methodFilter}
            onChange={(e) => setMethodFilter(e.target.value)}
            className="rounded-xl border border-slate-700/80 bg-slate-950/60 px-3 py-2 text-sm text-slate-200"
            aria-label="HTTP method filter"
            data-testid="openapi-method-filter"
          >
            <option value="all">All methods</option>
            <option value="GET">GET</option>
            <option value="POST">POST</option>
            <option value="PUT">PUT</option>
            <option value="DELETE">DELETE</option>
            <option value="PATCH">PATCH</option>
          </select>
        }
      />

      <div className="dash-card mb-6">
        <div className="flex items-center gap-3 mb-2">
          <BookOpen className="w-5 h-5 text-aether" />
          <h2 className="text-lg font-semibold text-slate-100">
            {doc?.info?.title ?? 'Aether API'} {doc?.info?.version ? `v${doc.info.version}` : ''}
          </h2>
        </div>
        {doc?.info?.description && (
          <p className="text-sm text-slate-400 mb-4">{doc.info.description}</p>
        )}
        <p className="text-xs text-slate-500">
          Raw document: <code className="text-aether/90">GET /api/openapi.json</code>
        </p>
        <button
          type="button"
          data-testid="openapi-download-json"
          onClick={downloadOpenApiJson}
          className="mt-3 inline-flex items-center gap-2 rounded-xl border border-slate-700 px-3 py-1.5 text-xs text-slate-300 hover:border-aether/40"
        >
          <Copy size={14} />
          Download JSON
        </button>
      </div>

      <div className="dash-card overflow-hidden mb-6">
        <h3 className="text-sm font-semibold text-slate-200 mb-4 px-1">Notable routes ({paths.length})</h3>
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-slate-800 text-left text-xs uppercase tracking-wider text-slate-500">
                <th className="py-2 px-4">Method</th>
                <th className="py-2 px-4">Path</th>
                <th className="py-2 px-4">Summary</th>
                <th className="py-2 px-4 w-16">Copy</th>
              </tr>
            </thead>
            <tbody>
              {paths.map((row) => (
                <tr key={`${row.method}-${row.path}`} className="border-b border-slate-800/50 hover:bg-slate-800/20">
                  <td className="py-2 px-4 font-mono text-xs text-aether">{row.method}</td>
                  <td className="py-2 px-4 font-mono text-xs text-slate-300">{row.path}</td>
                  <td className="py-2 px-4 text-slate-400">{row.summary || '—'}</td>
                  <td className="py-2 px-4">
                    <button
                      type="button"
                      data-testid={`openapi-copy-${row.method}-${row.path}`}
                      onClick={() => void copyPath(row.path)}
                      className="rounded-lg p-1.5 text-slate-500 hover:bg-slate-800 hover:text-aether"
                      title="Copy path"
                    >
                      <Copy size={14} />
                    </button>
                    {copiedPath === row.path ? (
                      <span className="ml-1 text-[10px] text-emerald-400">Copied</span>
                    ) : null}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          {paths.length === 0 && (
            <p className="text-sm text-slate-500 py-6 text-center">No routes match your search.</p>
          )}
        </div>
      </div>

      {doc && (
        <div className="dash-card">
          <h3 className="text-sm font-semibold text-slate-200 mb-3">Full OpenAPI JSON</h3>
          <CodeBlock title="openapi.json">{JSON.stringify(doc, null, 2)}</CodeBlock>
        </div>
      )}
    </div>
  );
}
