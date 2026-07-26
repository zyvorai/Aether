// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

/** Escape a CSV field (RFC 4180-ish). */
export function csvEscape(value: string): string {
  if (/[",\n\r]/.test(value)) {
    return `"${value.replace(/"/g, '""')}"`;
  }
  return value;
}

export interface WorkloadCsvRow {
  name: string;
  kind?: string | null;
  runtime: string;
  image: string;
  status: string;
  cluster?: string | null;
  namespace?: string | null;
  source?: string | null;
  created_at?: string | null;
}

/** Build a CSV document for the visible workloads table. */
export function workloadsToCsv(rows: WorkloadCsvRow[]): string {
  const header = ['name', 'kind', 'runtime', 'image', 'status', 'cluster', 'namespace', 'source', 'created_at'];
  const lines = [
    header.join(','),
    ...rows.map((row) =>
      [
        row.name,
        row.kind ?? '',
        row.runtime,
        row.image,
        row.status,
        row.cluster ?? '',
        row.namespace ?? '',
        row.source ?? 'aether',
        row.created_at ?? '',
      ]
        .map((cell) => csvEscape(String(cell)))
        .join(','),
    ),
  ];
  return `${lines.join('\n')}\n`;
}

export function downloadTextFile(filename: string, contents: string, mime = 'text/csv'): void {
  const blob = new Blob([contents], { type: mime });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = filename;
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  URL.revokeObjectURL(url);
}
