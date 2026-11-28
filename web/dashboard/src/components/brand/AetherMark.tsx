// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { MARK_PATH, MARK_TIGHT_VIEWBOX, MARK_VIEWBOX } from './geometry';

export interface AetherMarkProps {
  /** Rendered height in pixels. */
  size?: number;
  /** Crop to the ink so the mark can sit flush against type. */
  tight?: boolean;
  /** Accessible name. Without one the mark is decorative. */
  title?: string;
  className?: string;
}

/** The bare Aether mark. It takes the surrounding text colour. */
export default function AetherMark({ size = 24, tight = false, title, className }: AetherMarkProps) {
  return (
    <svg
      viewBox={tight ? MARK_TIGHT_VIEWBOX : MARK_VIEWBOX}
      width={tight ? (size * 56) / 48 : size}
      height={size}
      fill="currentColor"
      className={className}
      role={title ? 'img' : undefined}
      aria-label={title}
      aria-hidden={title ? undefined : true}
      focusable="false"
    >
      <path fillRule="evenodd" d={MARK_PATH} />
    </svg>
  );
}
