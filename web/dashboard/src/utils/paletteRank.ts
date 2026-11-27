// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

/** Search and ordering for the command palette. */

export interface Searchable {
  label: string;
  searchText: string;
}

export type PaletteScope = 'all' | 'actions' | 'workloads' | 'pages';

const SCOPE_PREFIX: Record<string, PaletteScope> = { '>': 'actions', '@': 'workloads', '/': 'pages' };

/** A leading `>`, `@` or `/` narrows the search to actions, workloads or pages. */
export function parseQuery(raw: string): { scope: PaletteScope; text: string } {
  const trimmed = raw.trimStart();
  const scope = SCOPE_PREFIX[trimmed.charAt(0)];
  return scope ? { scope, text: trimmed.slice(1).trim() } : { scope: 'all', text: trimmed.trim() };
}

const startsWord = (text: string, term: string) =>
  text.startsWith(term) || text.includes(` ${term}`) || text.includes(`-${term}`) || text.includes(`/${term}`);

/** Letters of `term` in order within `text`, all inside a short span. Catches "wrklds". */
function looseMatch(text: string, term: string): boolean {
  if (term.length < 3) return false;
  for (let start = text.indexOf(term[0]); start >= 0; start = text.indexOf(term[0], start + 1)) {
    if (start > 0 && !' -/'.includes(text[start - 1])) continue;
    let at = start;
    let matched = 1;
    while (matched < term.length) {
      at = text.indexOf(term[matched], at + 1);
      if (at < 0) break;
      matched += 1;
    }
    if (matched === term.length && at - start < term.length * 2) return true;
  }
  return false;
}

/**
 * 0 means no match. Every typed word has to be found; words found in the label
 * weigh far more than words found only in the extra search text.
 */
export function scoreCommand(query: string, command: Searchable): number {
  const terms = query.toLowerCase().split(/\s+/).filter(Boolean);
  if (terms.length === 0) return 1;
  const label = command.label.toLowerCase();
  const extra = command.searchText.toLowerCase();
  let score = 0;
  for (const term of terms) {
    if (startsWord(label, term)) score += 40;
    else if (label.includes(term)) score += 25;
    else if (startsWord(extra, term)) score += 8;
    else if (extra.includes(term)) score += 5;
    else if (looseMatch(label, term)) score += 3;
    else return 0;
  }
  const phrase = terms.join(' ');
  if (label === phrase || label.endsWith(` ${phrase}`)) score += 40;
  else if (label.includes(phrase)) score += 15;
  // Between equal matches the shorter label is the closer one.
  return score + Math.max(0, 10 - Math.floor(label.length / 8));
}

/**
 * Keeps each category together and orders the categories by their best match,
 * so a heading never appears twice. Input must already be sorted by score.
 */
export function groupByCategory<T extends { category: string }>(ranked: T[]): T[] {
  const groups = new Map<string, T[]>();
  for (const item of ranked) {
    const group = groups.get(item.category);
    if (group) group.push(item);
    else groups.set(item.category, [item]);
  }
  return [...groups.values()].flat();
}
