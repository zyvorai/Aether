// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useRef, useCallback } from 'react';
import { apiFetch } from '../utils/api';

interface LogViewerProps {
  workloadName: string;
  logsPath?: string;
}

export default function LogViewer({ workloadName, logsPath }: LogViewerProps) {
  const [logs, setLogs] = useState<string[]>([]);
  const [following, setFollowing] = useState(true);
  const [filter, setFilter] = useState('');
  const containerRef = useRef<HTMLPreElement>(null);

  const fetchLogs = useCallback(async () => {
    try {
      const resp = await apiFetch<string>(logsPath ?? `/workloads/${workloadName}/logs`);
      if (resp && typeof resp === 'string') {
        setLogs(resp.split('\n'));
      } else if (resp) {
        setLogs([String(resp)]);
      }
    } catch { /* ignore */ }
  }, [logsPath, workloadName]);

  useEffect(() => {
    fetchLogs();
    const interval = setInterval(fetchLogs, 3000);
    return () => clearInterval(interval);
  }, [fetchLogs]);

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

  const filteredLogs = filter
    ? logs.filter(l => l.toLowerCase().includes(filter.toLowerCase()))
    : logs;

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
        <button
          type="button"
          onClick={() => setFollowing(!following)}
          className={`glass-tab tab-chip ${following ? 'glass-tab-active tab-chip-active !border-emerald-500/30 !bg-emerald-500/10 !text-emerald-300' : ''}`}
        >
          {following ? 'Following' : 'Follow'}
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
        <span className="text-xs text-slate-500">{filteredLogs.length} lines</span>
      </div>

      <pre
        ref={containerRef}
        onScroll={handleScroll}
        className="glass-code-block-body max-h-96 overflow-auto p-3 text-xs"
      >
        {filteredLogs.length === 0 ? (
          <span className="text-slate-500">No logs available. Press Refresh to reload.</span>
        ) : (
          filteredLogs.map((line, i) => (
            <div key={i} className="flex">
              <span className="mr-2 w-12 shrink-0 select-none border-r glass-divider pr-2 text-right text-slate-600">
                {i + 1}
              </span>
              <span className={getLineColor(line)}>{line}</span>
            </div>
          ))
        )}
      </pre>
    </div>
  );
}
