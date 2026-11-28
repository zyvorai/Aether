// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

export interface LiveInput {
  connected: boolean;
  paused: boolean;
  heldUpdates: number;
  lastRefreshed: Date;
  now: number;
}

export interface LiveState {
  tone: 'live' | 'paused' | 'offline';
  label: string;
  /** Accessible name of the status button. */
  description: string;
  explanation: string;
}

export function agoLabel(from: number, now: number): string {
  const seconds = Math.max(0, Math.round((now - from) / 1000));
  if (seconds < 10) return 'just now';
  if (seconds < 60) return `${seconds}s ago`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
  return `${Math.floor(seconds / 3600)}h ago`;
}

export function liveState({ connected, paused, heldUpdates, lastRefreshed, now }: LiveInput): LiveState {
  const ago = agoLabel(lastRefreshed.getTime(), now);
  if (paused) {
    return {
      tone: 'paused',
      label: heldUpdates > 0 ? `Paused, ${heldUpdates} held` : 'Paused',
      description: `Automatic updates paused. Last refresh ${ago}`,
      explanation: 'Dashboard refreshes are on hold. Panels that poll on their own, such as logs, keep running.',
    };
  }
  if (!connected) {
    return {
      tone: 'offline',
      label: 'Reconnecting',
      description: `Reconnecting live updates. Last refresh ${ago}`,
      explanation: 'The event stream is down. The dashboard still refreshes every 60 seconds.',
    };
  }
  return {
    tone: 'live',
    label: `Live, ${ago}`,
    description: `Live updates connected. Last refresh ${ago}`,
    explanation: 'Changes arrive over the event stream, with a full refresh every 60 seconds.',
  };
}
