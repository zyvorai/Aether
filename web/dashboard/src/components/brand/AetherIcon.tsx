// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { useId } from 'react';
import { ICON_GRADIENT, ICON_MARK_TRANSFORM, MARK_PATH, MARK_VIEWBOX } from './geometry';

export interface AetherIconProps {
  /** Rendered width and height in pixels. */
  size?: number;
  /** Accessible name. Without one the icon is decorative. */
  title?: string;
  className?: string;
}

/** The app icon: the mark in white on the Aether blue tile. Same in both themes. */
export default function AetherIcon({ size = 64, title, className }: AetherIconProps) {
  const gradient = useId();
  return (
    <svg
      viewBox={MARK_VIEWBOX}
      width={size}
      height={size}
      className={className}
      role={title ? 'img' : undefined}
      aria-label={title}
      aria-hidden={title ? undefined : true}
      focusable="false"
    >
      <defs>
        <linearGradient id={gradient} x1="0.2" y1="0" x2="0.8" y2="1">
          <stop offset="0" stopColor={ICON_GRADIENT.from} />
          <stop offset="1" stopColor={ICON_GRADIENT.to} />
        </linearGradient>
      </defs>
      <rect width="64" height="64" rx="14.4" fill={`url(#${gradient})`} />
      <path fill="#fff" fillRule="evenodd" d={MARK_PATH} transform={ICON_MARK_TRANSFORM} />
    </svg>
  );
}
