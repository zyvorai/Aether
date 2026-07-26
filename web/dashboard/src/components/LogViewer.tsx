// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useRef, useCallback } from 'react';
import { apiFetch } from '../utils/api';

interface LogViewerProps {
  workloadName: string;
  logsPath?: string;
  /** Available container names for the resolved pod (multi-container workloads). */
  containers?: string[];
}

const TAIL_OPTIONS = [100, 200, 500, 1000, 5000] as const;

/** Append/replace a query param on a relative API path without a full URL parse. */
function withParam(path: string, key: string, value: string): string {
  const [base, existing] = path.split('?');
  const params = new URLSearchParams(existing ?? '');
  params.set(key, value);
  return `${base}?${params.toString()}`;
}

export default function LogViewer({ workloadName, logsPath, containers = [] }: LogViewerProps) {
  const [logs, setLogs] = useState<string[]>([]);
  const [following, setFollowing] = useState(true);
  const [autoRefresh, setAutoRefresh] = useState(true);
  const [wrap, setWrap] = useState(false);
  const [filter, setFilter] = useState('');
  const [tail, setTail] = useState<number>(200);
  const [container, setContainer] = useState('');
  const [lastUpdated, setLastUpdated] = useState<number | null>(null);
  const containerRef = useRef<HTMLPreElement>(null);

  useEffect(() => {
    // Reset container selection when the available set changes.
    if (containers.length === 0) {
      setContainer('');
      return;
    }
    if (!container || !containers.includes(container)) {
      setContainer(containers[0] ?? '');
    }
  }, [containers, container]);

  const fetchLogs = useCallback(async () => {
    try {
      const basePath = logsPath ?? `/workloads/${workloadName}/logs`;
      let path = withParam(basePath, 'tail', String(tail));
      if (container) {
        path = withParam(path, 'container', container);
      }
      const resp = await apiFetch<string>(path);
      if (resp && typeof resp === 'string') {
        setLogs(resp.split('\n'));
      } else if (resp) {
        setLogs([String(resp)]);
      }
      setLastUpdated(Date.now());
    } catch { /* ignore */ }
  }, [logsPath, workloadName, tail, container]);

  useEffect(() => {
    fetchLogs();
    if (!autoRefresh) return;
    const interval = setInterval(fetchLogs, 3000);
    return () => clearInterval(interval);
  }, [fetchLogs, autoRefresh]);

  useEffect(() => {
    if (following && containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [logs, following]);

  const handleScroll = () => {
    if (!containerRef.current) return;
    const { scrollTop, scrollHeight, clientHeight } = containerRef.current;
    const atBottom = scrollHeight - scrollTop - clientHeight < 50;
    if (!atBottom && following) setFollowing(false);
    if (atBottom && !following) setFollowing(true);
  };

  const jumpToBottom = () => {
    if (containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
    setFollowing(true);
  };

  const filteredLogs = filter
    ? logs.filter(l => l.toLowerCase().includes(filter.toLowerCase()))
    : logs;

  const downloadLogs = () => {
    const blob = new Blob([filteredLogs.join('\n')], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement('a');
    const stamp = new Date().toISOString().replace(/[:.]/g, '-');
    anchor.href = url;
    anchor.download = `${workloadName.replace(/[^a-zA-Z0-9._-]/g, '_')}-${stamp}.log`;
    document.body.appendChild(anchor);
    anchor.click();
    anchor.remove();
    URL.revokeObjectURL(url);
  };

  const getLineColor = (line: string) => {
    const lower = line.toLowerCase();
    if (lower.includes('error') || lower.includes('fatal') || lower.includes('panic')) return 'text-red-400';
    if (lower.includes('warn')) return 'text-orange-400';
    if (lower.includes('info')) return 'text-green-400';
    if (lower.includes('debug') || lower.includes('trace')) return 'text-slate-500';
    return 'text-slate-300';
  };

  return (
    <div className="glass-panel-card p-4" data-testid="log-viewer">
      <div className="mb-3 flex flex-wrap items-center gap-2">
        <input
          type="text"
          placeholder="Filter logs..."
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          className="glass-input min-w-[12rem] flex-1"
        />
        {containers.length > 1 ? (
          <label className="flex items-center gap-1 text-xs text-slate-400">
            Container
            <select
              value={container}
              onChange={(e) => setContainer(e.target.value)}
              className="glass-input !py-1 !px-2 text-xs max-w-[10rem]"
              data-testid="log-container-select"
              aria-label="Container"
            >
              {containers.map((name) => (
                <option key={name} value={name}>{name}</option>
              ))}
            </select>
          </label>
        ) : null}
        <label className="flex items-center gap-1 text-xs text-slate-400">
          Tail
          <select
            value={tail}
            onChange={(e) => setTail(Number(e.target.value))}
            className="glass-input !py-1 !px-2 text-xs"
            data-testid="log-tail-select"
            aria-label="Tail line count"
          >
            {TAIL_OPTIONS.map((n) => (
              <option key={n} value={n}>{n}</option>
            ))}
          </select>
        </label>
        <button
          type="button"
          onClick={() => setFollowing(!following)}
          className={`glass-tab tab-chip ${following ? 'glass-tab-active tab-chip-active !border-emerald-500/30 !bg-emerald-500/10 !text-emerald-300' : ''}`}
        >
          {following ? 'Following' : 'Follow'}
        </button>
        <button
          type="button"
          onClick={() => setAutoRefresh(!autoRefresh)}
          className={`glass-tab tab-chip ${autoRefresh ? 'glass-tab-active tab-chip-active' : ''}`}
          data-testid="log-autorefresh-toggle"
          title={autoRefresh ? 'Auto-refresh on (3s)' : 'Auto-refresh paused'}
        >
          {autoRefresh ? 'Live' : 'Paused'}
        </button>
        <button
          type="button"
          onClick={() => setWrap(!wrap)}
          className={`glass-tab tab-chip ${wrap ? 'glass-tab-active tab-chip-active' : ''}`}
          title="Toggle line wrapping"
        >
          Wrap
        </button>
        <button type="button" onClick={fetchLogs} className="btn-secondary !px-3 !py-1.5 !text-xs">
          Refresh
        </button>
        <button
          type="button"
          onClick={() => { void navigator.clipboard.writeText(filteredLogs.join('\n')); }}
          className="btn-secondary !px-3 !py-1.5 !text-xs"
        >
          Copy
        </button>
        <button
          type="button"
          onClick={downloadLogs}
          className="btn-secondary !px-3 !py-1.5 !text-xs"
          data-testid="log-download"
        >
          Download
        </button>
        <span className="text-xs text-slate-500">
          {filteredLogs.length} lines
          {lastUpdated ? ` · ${new Date(lastUpdated).toLocaleTimeString()}` : ''}
        </span>
      </div>

      <div className="relative">
        <pre
          ref={containerRef}
          onScroll={handleScroll}
          className={`glass-code-block-body max-h-96 overflow-auto p-3 text-xs ${wrap ? 'whitespace-pre-wrap break-words' : ''}`}
        >
          {filteredLogs.length === 0 ? (
            <span className="text-slate-500">No logs available. Press Refresh to reload.</span>
          ) : (
            filteredLogs.map((line, i) => (
              <div key={i} className="flex">
                <span className="mr-2 w-12 shrink-0 select-none border-r glass-divider pr-2 text-right text-slate-600">
                  {i + 1}
                </span>
                <span className={`${getLineColor(line)} ${wrap ? 'min-w-0 break-words' : ''}`}>{line}</span>
              </div>
            ))
          )}
        </pre>
        {!following ? (
          <button
            type="button"
            onClick={jumpToBottom}
            className="absolute bottom-3 right-3 rounded-full glass-inset-surface px-3 py-1.5 text-xs text-slate-200 shadow-lg hover:text-aether"
            data-testid="log-jump-bottom"
          >
            ↓ Jump to latest
          </button>
        ) : null}
      </div>
    </div>
  );
}
