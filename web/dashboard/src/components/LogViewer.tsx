// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useRef, useCallback } from 'react';
import { apiFetchWithError } from '../utils/api';
import { copyToClipboard } from '../utils/clipboard';

interface LogViewerProps {
  workloadName: string;
  logsPath?: string;
  /** Available container names for the resolved pod (multi-container workloads). */
  containers?: string[];
  /** Optional status hint for empty-state messaging (e.g. Succeeded / stopped). */
  statusHint?: string;
}

const TAIL_OPTIONS = [100, 200, 500, 1000, 5000] as const;

/** Append/replace a query param on a relative API path without a full URL parse. */
function withParam(path: string, key: string, value: string): string {
  const [base, existing] = path.split('?');
  const params = new URLSearchParams(existing ?? '');
  params.set(key, value);
  return `${base}?${params.toString()}`;
}

export default function LogViewer({ workloadName, logsPath, containers = [], statusHint }: LogViewerProps) {
  const [logs, setLogs] = useState<string[]>([]);
  const [following, setFollowing] = useState(true);
  const [autoRefresh, setAutoRefresh] = useState(true);
  const [wrap, setWrap] = useState(false);
  const [filter, setFilter] = useState('');
  const [tail, setTail] = useState<number>(200);
  const [container, setContainer] = useState('');
  const [previous, setPrevious] = useState(false);
  const [fetchError, setFetchError] = useState('');
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
      if (previous) {
        path = withParam(path, 'previous', 'true');
      }
      const { data: resp, error } = await apiFetchWithError<string>(path);
      if (typeof resp === 'string') {
        // Empty string is a valid response (completed pods with no output).
        setLogs(resp.length > 0 ? resp.split('\n') : []);
        setFetchError('');
      } else if (resp == null) {
        setFetchError(
          error && /no pods found/i.test(error)
            ? "No pods found for this workload — they've likely already been cleaned up by Kubernetes."
            : error || 'Failed to load logs from the API.',
        );
      } else {
        setLogs([String(resp)]);
        setFetchError('');
      }
      setLastUpdated(Date.now());
    } catch {
      setFetchError('Failed to load logs.');
    }
  }, [logsPath, workloadName, tail, container, previous]);

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

  const nonemptyFiltered = filteredLogs.filter((line) => line.length > 0);

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

  const emptyMessage = fetchError
    || (previous
      ? 'No previous container logs available.'
      : statusHint && /stop|succeed|complet|exit|fail/i.test(statusHint)
        ? `No current logs for this ${statusHint.toLowerCase()} workload. Try “Previous” if the container restarted, or check Events.`
        : 'No logs available yet. Press Refresh to reload, or enable Previous for the last terminated instance.');

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
          onClick={() => setPrevious(!previous)}
          className={`glass-tab tab-chip ${previous ? 'glass-tab-active tab-chip-active !border-amber-500/30 !bg-amber-500/10 !text-amber-300' : ''}`}
          data-testid="log-previous-toggle"
          title="Show logs from the previous container instance"
        >
          Previous
        </button>
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
          onClick={() => { void copyToClipboard(filteredLogs.join('\n')); }}
          className="btn-secondary !px-3 !py-1.5 !text-xs"
          disabled={nonemptyFiltered.length === 0}
        >
          Copy
        </button>
        <button
          type="button"
          onClick={downloadLogs}
          className="btn-secondary !px-3 !py-1.5 !text-xs"
          data-testid="log-download"
          disabled={nonemptyFiltered.length === 0}
        >
          Download
        </button>
        <span className="text-xs text-slate-500">
          {nonemptyFiltered.length} lines
          {lastUpdated ? ` · ${new Date(lastUpdated).toLocaleTimeString()}` : ''}
        </span>
      </div>

      <div className="relative">
        <pre
          ref={containerRef}
          onScroll={handleScroll}
          className={`glass-code-block-body max-h-[min(32rem,60vh)] overflow-auto p-3 text-xs ${wrap ? 'whitespace-pre-wrap break-words' : ''}`}
          data-testid="log-viewer-body"
        >
          {nonemptyFiltered.length === 0 ? (
            <span className="text-slate-500" data-testid="log-viewer-empty">{emptyMessage}</span>
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
        {!following && nonemptyFiltered.length > 0 ? (
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
