// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useEffect, useRef } from 'react';
import { EditorState } from '@codemirror/state';
import { EditorView, keymap, lineNumbers, highlightActiveLine, highlightActiveLineGutter } from '@codemirror/view';
import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';
import { yaml } from '@codemirror/lang-yaml';

interface YamlCodeEditorProps {
  value: string;
  onChange: (value: string) => void;
  heightPx: number;
  placeholder?: string;
  readOnly?: boolean;
  'aria-label'?: string;
}

function editorTheme() {
  return EditorView.theme(
    {
      '&': {
        fontSize: '14px',
        fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
      },
      '.cm-content': {
        padding: '16px 0',
        caretColor: '#f97316',
      },
      '.cm-line': {
        padding: '0 16px',
        lineHeight: '24px',
      },
      '.cm-gutters': {
        borderRight: '1px solid #3f3f46',
        paddingRight: '4px',
      },
      '.cm-gutterElement': {
        padding: '0 8px 0 12px',
        minWidth: '2.5rem',
      },
      '&.cm-focused .cm-cursor': {
        borderLeftColor: '#f97316',
      },
      '&.cm-focused .cm-selectionBackground, ::selection': {
        backgroundColor: '#ea580c40',
      },
    },
    { dark: true },
  );
}

function surfaceTheme() {
  return EditorView.theme({
    '&': {
      height: '100%',
      backgroundColor: '#09090b',
      color: '#d4d4d8',
      border: '1px solid #3f3f46',
      borderRadius: '12px',
      overflow: 'hidden',
    },
    '.cm-scroller': {
      overflow: 'auto',
      fontFamily: 'inherit',
    },
    '.cm-gutters': {
      backgroundColor: '#18181b',
      color: '#71717a',
    },
    '.cm-activeLineGutter': {
      backgroundColor: '#27272a',
    },
    '.cm-activeLine': {
      backgroundColor: '#ea580c12',
    },
  });
}

export default function YamlCodeEditor({
  value,
  onChange,
  heightPx,
  placeholder,
  readOnly,
  'aria-label': ariaLabel,
}: YamlCodeEditorProps) {
  const hostRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;

  useEffect(() => {
    if (!hostRef.current) return;

    const updateListener = EditorView.updateListener.of((update) => {
      if (update.docChanged) {
        onChangeRef.current(update.state.doc.toString());
      }
    });

    const state = EditorState.create({
      doc: value,
      extensions: [
        lineNumbers(),
        highlightActiveLine(),
        highlightActiveLineGutter(),
        history(),
        yaml(),
        keymap.of([...defaultKeymap, ...historyKeymap]),
        editorTheme(),
        surfaceTheme(),
        updateListener,
        EditorView.lineWrapping,
        EditorState.readOnly.of(!!readOnly),
        EditorView.contentAttributes.of({ 'aria-label': ariaLabel ?? 'Workload YAML' }),
      ],
    });

    const view = new EditorView({ state, parent: hostRef.current });
    viewRef.current = view;

    return () => {
      view.destroy();
      viewRef.current = null;
    };
  }, [readOnly, ariaLabel]);

  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    const current = view.state.doc.toString();
    if (current !== value) {
      view.dispatch({
        changes: { from: 0, to: current.length, insert: value },
      });
    }
  }, [value]);

  return (
    <div
      data-testid="yaml-editor"
      className="relative w-full shrink-0"
      style={{ height: `${heightPx}px` }}
    >
      {!value && placeholder ? (
        <div className="pointer-events-none absolute left-14 top-4 z-10 text-sm font-mono text-zinc-500">
          {placeholder}
        </div>
      ) : null}
      <div ref={hostRef} className="h-full w-full" />
    </div>
  );
}
