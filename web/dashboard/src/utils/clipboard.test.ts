// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { afterEach, describe, expect, it, vi } from 'vitest';
import { copyToClipboard } from './clipboard';

// This project's vitest environment is 'node' (no jsdom) — stub just enough of
// `document` for the execCommand fallback path, matching the manual-stub
// convention already used for `localStorage` in pinnedWorkloads.test.ts.
function stubDocument(execCommandImpl: () => boolean) {
  const textarea = {
    value: '',
    style: {} as Record<string, string>,
    setAttribute: vi.fn(),
    select: vi.fn(),
    setSelectionRange: vi.fn(),
  };
  const body = { appendChild: vi.fn(), removeChild: vi.fn() };
  const execCommand = vi.fn(execCommandImpl);
  vi.stubGlobal('document', {
    createElement: vi.fn(() => textarea),
    body,
    execCommand,
  });
  return { textarea, body, execCommand };
}

describe('copyToClipboard', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('uses navigator.clipboard.writeText when available (secure context)', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    vi.stubGlobal('navigator', { clipboard: { writeText } });

    const ok = await copyToClipboard('hello');

    expect(ok).toBe(true);
    expect(writeText).toHaveBeenCalledWith('hello');
  });

  it('falls back to document.execCommand when navigator.clipboard is undefined (plain HTTP)', async () => {
    vi.stubGlobal('navigator', {});
    const { execCommand, body, textarea } = stubDocument(() => true);

    const ok = await copyToClipboard('hello');

    expect(ok).toBe(true);
    expect(textarea.value).toBe('hello');
    expect(execCommand).toHaveBeenCalledWith('copy');
    expect(body.appendChild).toHaveBeenCalledWith(textarea);
    expect(body.removeChild).toHaveBeenCalledWith(textarea);
  });

  it('never throws even if every copy path fails', async () => {
    vi.stubGlobal('navigator', {});
    stubDocument(() => {
      throw new Error('denied');
    });

    await expect(copyToClipboard('hello')).resolves.toBe(false);
  });
});
