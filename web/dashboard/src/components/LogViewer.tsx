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

  // Initial fetch + polling every 3s
  useEffect(() => {
    fetchLogs();
    const interval = setInterval(fetchLogs, 3000);
    return () => clearInterval(interval);
  }, [fetchLogs]);

  // Auto-scroll to bottom
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
    if (lower.includes('debug') || lower.includes('trace')) return 'text-zinc-500';
    return 'text-zinc-300';
  };

  return (
    <div>
      {/* Controls */}
      <div className="flex items-center gap-2 mb-2">
        <input
          type="text"
          placeholder="Filter logs..."
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          className="bg-zinc-800 border border-zinc-600 rounded px-2 py-1 text-sm text-white flex-1"
        />
        <button
          onClick={() => setFollowing(!following)}
          className={`px-2 py-1 text-xs rounded border ${
            following ? 'bg-emerald-600/20 text-emerald-400 border-emerald-600/30' : 'bg-zinc-700 text-zinc-400 border-zinc-600'
          }`}
        >
          {following ? 'Following' : 'Follow'}
        </button>
        <button
          onClick={fetchLogs}
          className="px-2 py-1 text-xs rounded bg-zinc-700 text-zinc-300 border border-zinc-600 hover:bg-zinc-600"
        >
          Refresh
        </button>
        <button
          onClick={() => { navigator.clipboard.writeText(filteredLogs.join('\n')); }}
          className="px-2 py-1 text-xs rounded bg-zinc-700 text-zinc-300 border border-zinc-600 hover:bg-zinc-600"
        >
          Copy
        </button>
        <span className="text-xs text-zinc-500">{filteredLogs.length} lines</span>
      </div>

      {/* Log Output */}
      <pre
        ref={containerRef}
        onScroll={handleScroll}
        className="bg-zinc-950 rounded p-3 text-xs font-mono overflow-auto max-h-96 border border-zinc-700"
      >
        {filteredLogs.length === 0 ? (
          <span className="text-zinc-500">No logs available. Press Refresh to reload.</span>
        ) : (
          filteredLogs.map((line, i) => (
            <div key={i} className="flex">
              <span className="text-zinc-600 select-none w-12 text-right pr-2 border-r border-zinc-800 mr-2">
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
