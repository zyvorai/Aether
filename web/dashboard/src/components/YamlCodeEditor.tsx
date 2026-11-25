// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { useEffect, useRef } from 'react';
import { EditorState } from '@codemirror/state';
import { EditorView, keymap, lineNumbers, highlightActiveLine, highlightActiveLineGutter } from '@codemirror/view';
import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';
import { yaml } from '@codemirror/lang-yaml';
import { HighlightStyle, syntaxHighlighting } from '@codemirror/language';
import { tags as t } from '@lezer/highlight';

// Syntax colours come from theme.css so the editor follows light and dark.
const terminalHighlightStyle = HighlightStyle.define([
  { tag: t.propertyName, color: 'var(--code-key)' },
  { tag: t.string, color: 'var(--code-string)' },
  { tag: [t.number, t.integer, t.float], color: 'var(--code-number)' },
  { tag: [t.bool, t.null, t.atom, t.keyword], color: 'var(--code-atom)' },
  { tag: t.comment, color: 'var(--code-comment)', fontStyle: 'italic' },
  { tag: [t.punctuation, t.separator, t.meta], color: 'var(--code-punct)' },
  { tag: t.invalid, color: 'var(--code-invalid)' },
]);

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
        caretColor: 'var(--primary)',
      },
      '.cm-line': {
        padding: '0 16px',
        lineHeight: '24px',
      },
      '.cm-gutters': {
        borderRight: '1px solid var(--border)',
        paddingRight: '4px',
      },
      '.cm-gutterElement': {
        padding: '0 8px 0 12px',
        minWidth: '2.5rem',
      },
      '&.cm-focused .cm-cursor': {
        borderLeftColor: 'var(--primary)',
      },
      '&.cm-focused .cm-selectionBackground, ::selection': {
        backgroundColor: 'var(--code-selection)',
      },
    },
  );
}

function surfaceTheme() {
  return EditorView.theme({
    '&': {
      height: '100%',
      backgroundColor: 'var(--code-bg)',
      color: 'var(--code-fg)',
      border: '1px solid var(--border)',
      borderRadius: '12px',
      overflow: 'hidden',
    },
    '.cm-scroller': {
      overflow: 'auto',
      fontFamily: 'inherit',
    },
    '.cm-gutters': {
      backgroundColor: 'var(--code-gutter)',
      color: 'var(--code-gutter-fg)',
    },
    '.cm-activeLineGutter': {
      backgroundColor: 'var(--code-active)',
    },
    '.cm-activeLine': {
      backgroundColor: 'var(--code-active)',
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
        syntaxHighlighting(terminalHighlightStyle),
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
    // The scroll container must be reachable from the keyboard.
    view.scrollDOM.tabIndex = 0;

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
        <div className="pointer-events-none absolute left-14 top-4 z-10 text-sm font-mono text-subtle">
          {placeholder}
        </div>
      ) : null}
      <div ref={hostRef} className="h-full w-full" />
    </div>
  );
}
