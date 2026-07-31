// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { forwardRef, useEffect, useImperativeHandle, useRef } from 'react';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';

export interface ExecTerminalHandle {
  write: (data: string) => void;
  writeln: (data: string) => void;
  clear: () => void;
  focus: () => void;
  getBufferText: () => string;
}

interface ExecTerminalProps {
  /** When true, keystrokes are forwarded via onData. */
  interactive: boolean;
  onData?: (data: string) => void;
  className?: string;
}

const ExecTerminal = forwardRef<ExecTerminalHandle, ExecTerminalProps>(function ExecTerminal(
  { interactive, onData, className },
  ref,
) {
  const hostRef = useRef<HTMLDivElement | null>(null);
  const termRef = useRef<Terminal | null>(null);
  const fitRef = useRef<FitAddon | null>(null);
  const onDataRef = useRef(onData);
  const interactiveRef = useRef(interactive);

  onDataRef.current = onData;
  interactiveRef.current = interactive;

  useImperativeHandle(ref, () => ({
    write(data: string) {
      termRef.current?.write(data);
    },
    writeln(data: string) {
      termRef.current?.writeln(data);
    },
    clear() {
      termRef.current?.clear();
      termRef.current?.reset();
    },
    focus() {
      termRef.current?.focus();
    },
    getBufferText() {
      const term = termRef.current;
      if (!term) return '';
      const buffer = term.buffer.active;
      const lines: string[] = [];
      for (let i = 0; i < buffer.length; i += 1) {
        lines.push(buffer.getLine(i)?.translateToString(true) ?? '');
      }
      return lines.join('\n').replace(/\n+$/, '\n');
    },
  }));

  useEffect(() => {
    const host = hostRef.current;
    if (!host) return;

    const term = new Terminal({
      cursorBlink: true,
      convertEol: true,
      fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
      fontSize: 13,
      lineHeight: 1.35,
      theme: {
        background: '#0b1220',
        foreground: '#34d399',
        cursor: '#34d399',
        selectionBackground: 'rgba(52, 211, 153, 0.25)',
      },
      allowProposedApi: true,
    });
    const fit = new FitAddon();
    term.loadAddon(fit);
    term.open(host);
    fit.fit();
    termRef.current = term;
    fitRef.current = fit;

    const disposable = term.onData((data) => {
      if (!interactiveRef.current) return;
      onDataRef.current?.(data);
    });

    const ro = typeof ResizeObserver !== 'undefined'
      ? new ResizeObserver(() => {
          try {
            fit.fit();
          } catch {
            /* terminal not ready */
          }
        })
      : null;
    ro?.observe(host);

    const onWinResize = () => {
      try {
        fit.fit();
      } catch {
        /* ignore */
      }
    };
    window.addEventListener('resize', onWinResize);

    return () => {
      disposable.dispose();
      ro?.disconnect();
      window.removeEventListener('resize', onWinResize);
      term.dispose();
      termRef.current = null;
      fitRef.current = null;
    };
  }, []);

  useEffect(() => {
    try {
      fitRef.current?.fit();
    } catch {
      /* ignore */
    }
    if (interactive) {
      termRef.current?.focus();
    }
  }, [interactive, className]);

  return (
    <div
      ref={hostRef}
      data-testid="workload-shell-output"
      className={className ?? 'glass-code-block-body h-[420px] w-full overflow-hidden p-2 shadow-inner'}
    />
  );
});

export default ExecTerminal;
