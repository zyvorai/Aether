// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { WORDMARK_PATHS, WORDMARK_VIEWBOX } from './geometry';

export interface AetherWordmarkProps {
  /** Rendered height in pixels, ascender to just below the baseline. */
  height?: number;
  /** Accessible name. Without one the wordmark is decorative. */
  title?: string;
  className?: string;
}

const [, , WIDTH_UNITS, HEIGHT_UNITS] = WORDMARK_VIEWBOX.split(' ').map(Number);

/** "aether" drawn as outlines, in the surrounding text colour. */
export default function AetherWordmark({ height = 24, title, className }: AetherWordmarkProps) {
  return (
    <svg
      viewBox={WORDMARK_VIEWBOX}
      width={(height * WIDTH_UNITS) / HEIGHT_UNITS}
      height={height}
      fill="currentColor"
      className={className}
      role={title ? 'img' : undefined}
      aria-label={title}
      aria-hidden={title ? undefined : true}
      focusable="false"
    >
      {WORDMARK_PATHS.map((d, index) => (
        <path key={index} fillRule="evenodd" d={d} />
      ))}
    </svg>
  );
}
