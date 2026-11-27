// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

const MAX_MESSAGE = 240;

/** One line saying which request failed and how, for error states and bug reports. */
export function describeFetchFailure(path: string, status?: number, message?: string | null): string {
  const request = `GET /api${path}`;
  if (status === undefined) return `${request}: no response from the server`;
  const reason = message?.trim();
  const trimmed = reason && reason.length > MAX_MESSAGE ? `${reason.slice(0, MAX_MESSAGE)}…` : reason;
  return trimmed ? `${request}: HTTP ${status}, ${trimmed}` : `${request}: HTTP ${status}, unreadable response`;
}
