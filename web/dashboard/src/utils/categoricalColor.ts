// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

/*
  Deterministic-but-arbitrary color assignment for cluster/namespace/
  environment chips, drawn from the iPhone 17 categorical palette (see
  theme.css) plus --brand — the same name always gets the same color
  within a session, so a "prod" chip stays visually distinct from a
  "staging" chip wherever it appears, without any color carrying meaning
  (unlike severity red/amber/green, which stay reserved for status).
*/

const PALETTE = ['bg-brand', 'bg-deepblue', 'bg-sage', 'bg-mistblue', 'bg-lavender', 'bg-gold'] as const;

function hashString(value: string): number {
  let hash = 2166136261;
  for (let i = 0; i < value.length; i++) {
    hash ^= value.charCodeAt(i);
    hash = Math.imul(hash, 16777619);
  }
  return Math.abs(hash);
}

/** Tailwind bg-* class (solid fill) for a chip's small color dot. */
export function categoricalDotClass(name: string): string {
  return PALETTE[hashString(name) % PALETTE.length];
}
