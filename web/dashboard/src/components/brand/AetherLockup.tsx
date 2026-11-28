// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { cn } from '../../lib/cn';
import AetherMark from './AetherMark';
import AetherWordmark from './AetherWordmark';

export interface AetherLockupProps {
  /** Height of the mark in pixels; the wordmark scales with it. */
  size?: number;
  /** Accessible name. Without one the lockup is decorative. */
  title?: string;
  className?: string;
  /** Lets a narrow layout hide the wordmark and keep the mark. */
  wordmarkClassName?: string;
}

/**
 * Horizontal signature. The mark is as tall as the ascenders (140 units) and
 * shares their baseline; the gap is 46 units.
 */
export default function AetherLockup({ size = 20, title, className, wordmarkClassName }: AetherLockupProps) {
  const unit = size / 140;
  return (
    <span
      className={cn('inline-flex items-start', className)}
      style={{ columnGap: unit * 46 }}
      role={title ? 'img' : undefined}
      aria-label={title}
    >
      <AetherMark size={size} tight />
      <span className={cn('flex', wordmarkClassName)}>
        <AetherWordmark height={unit * 144} />
      </span>
    </span>
  );
}
