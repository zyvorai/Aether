// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState, useEffect, useRef, type ReactNode } from 'react';
import {
  Send,
  Loader2,
  Copy,
  RotateCcw,
  Upload,
  Maximize2,
  Minimize2,
  ClipboardCheck,
} from 'lucide-react';
import { countYamlLines, yamlEditorHeightPx } from '../utils/editorHeight';
import YamlCodeEditor from './YamlCodeEditor';

interface YamlInputProps {
  placeholder?: string;
  buttonText: string;
  onSubmit: (yaml: string) => void;
  loading?: boolean;
  initialValue?: string;
  value?: string;
  onChange?: (yaml: string) => void;
  resetValue?: string;
  layout?: 'compact' | 'editor';
  submitDisabled?: boolean;
  editorLoading?: boolean;
  onValidate?: (yaml: string) => void;
  validateLoading?: boolean;
  showValidateButton?: boolean;
  submitTestId?: string;
  header?: ReactNode;
  footer?: ReactNode;
}

export default function YamlInput({
  placeholder,
  buttonText,
  onSubmit,
  loading,
  initialValue,
  value: controlledValue,
  onChange,
  resetValue,
  layout = 'compact',
  submitDisabled,
  editorLoading,
  onValidate,
  validateLoading,
  showValidateButton,
  submitTestId,
  header,
  footer,
}: YamlInputProps) {
  const [internalValue, setInternalValue] = useState(initialValue ?? '');
  const isControlled = controlledValue !== undefined;
  const value = isControlled ? controlledValue : internalValue;
  const setValue = (next: string) => {
    if (!isControlled) setInternalValue(next);
    onChange?.(next);
  };
  const [expanded, setExpanded] = useState(false);
  const [copyHint, setCopyHint] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const isEditor = layout === 'editor';
  const lineCount = countYamlLines(value);
  const editorHeight = yamlEditorHeightPx(lineCount, expanded);
  const resetTarget = resetValue ?? initialValue ?? '';

  useEffect(() => {
    if (!isControlled) {
      setInternalValue(initialValue ?? '');
    }
  }, [initialValue, isControlled]);

  function handleSubmit() {
    const trimmed = value.trim();
    if (trimmed && !loading && !submitDisabled && !editorLoading) {
      onSubmit(trimmed);
    }
  }

  function handleValidateClick() {
    const trimmed = value.trim();
    if (trimmed && onValidate && !validateLoading) {
      onValidate(trimmed);
    }
  }

  async function handleCopy() {
    try {
      await navigator.clipboard.writeText(value);
      setCopyHint('Copied');
      setTimeout(() => setCopyHint(null), 2000);
    } catch {
      setCopyHint('Copy failed');
      setTimeout(() => setCopyHint(null), 2000);
    }
  }

  function handleReset() {
    setValue(resetTarget);
  }

  function handleFileUpload(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => {
      if (typeof reader.result === 'string') {
        setValue(reader.result);
      }
    };
    reader.readAsText(file);
    e.target.value = '';
  }

  const toolbarBtn =
    'inline-flex items-center gap-1.5 rounded-lg border border-slate-800/60 bg-[#11151C]/80 px-2.5 py-1.5 text-xs font-medium text-slate-400 transition-colors hover:bg-slate-800/60 hover:text-slate-200 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-aether/40 disabled:cursor-not-allowed disabled:opacity-40';

  const textareaClass =
    'w-full rounded-xl border border-slate-800/60 bg-[#0B0E14] p-4 text-sm font-mono leading-relaxed text-slate-300 placeholder-slate-600 resize-y transition-colors focus:border-aether focus:outline-none focus:ring-1 focus:ring-aether/30';

  return (
    <div className={isEditor ? 'flex min-h-0 flex-1 flex-col gap-3 overflow-hidden' : 'space-y-3'}>
      {header}
      {isEditor ? (
        <>
          <EditorToolbar
            toolbarBtn={toolbarBtn}
            copyHint={copyHint}
            onCopy={handleCopy}
            onReset={handleReset}
            resetDisabled={!resetTarget}
            onUploadClick={() => fileInputRef.current?.click()}
            expanded={expanded}
            onToggleExpand={() => setExpanded((v) => !v)}
            lineCount={lineCount}
            hasValue={!!value}
          />
          <input
            ref={fileInputRef}
            type="file"
            accept=".yaml,.yml,text/yaml"
            className="hidden"
            onChange={handleFileUpload}
          />

          <div className="relative min-h-0 flex-1 overflow-hidden">
            {editorLoading ? (
              <div className="absolute inset-0 z-10 flex items-center justify-center rounded-xl border border-slate-800/60 bg-[#0B0E14]/90">
                <Loader2 className="w-6 h-6 animate-spin text-aether" />
                <span className="ml-2 text-sm text-slate-400">Loading template…</span>
              </div>
            ) : null}
            <YamlCodeEditor
              value={value}
              onChange={setValue}
              heightPx={editorHeight}
              placeholder={placeholder ?? 'Paste YAML here...'}
              readOnly={editorLoading}
              aria-label="Workload YAML"
            />
          </div>
        </>
      ) : (
        <textarea
          value={value}
          onChange={(e) => setValue(e.target.value)}
          placeholder={placeholder ?? 'Paste YAML here...'}
          rows={10}
          className={textareaClass}
          spellCheck={false}
          aria-label="Workload YAML"
        />
      )}

      {footer}

      <div className={`relative z-10 flex flex-wrap items-center justify-end gap-2 ${isEditor ? 'shrink-0 bg-[#0B0E14] pt-2' : ''}`}>
        {showValidateButton && onValidate ? (
          <button
            type="button"
            onClick={handleValidateClick}
            disabled={!value.trim() || validateLoading || editorLoading}
            className="flex items-center gap-2 btn-secondary disabled:cursor-not-allowed disabled:opacity-40"
          >
            {validateLoading ? (
              <Loader2 className="w-4 h-4 animate-spin" />
            ) : (
              <ClipboardCheck className="w-4 h-4" />
            )}
            Validate
          </button>
        ) : null}
        <button
          type="button"
          data-testid={submitTestId ?? 'yaml-submit'}
          onClick={handleSubmit}
          disabled={!value.trim() || loading || submitDisabled || editorLoading}
          className={`flex shrink-0 items-center gap-2 btn-primary disabled:cursor-not-allowed disabled:opacity-40 ${isEditor ? '' : 'w-full sm:w-auto justify-center'}`}
        >
          {loading ? <Loader2 className="w-4 h-4 animate-spin" /> : <Send className="w-4 h-4" />}
          {buttonText}
        </button>
      </div>
    </div>
  );
}

function EditorToolbar({
  toolbarBtn,
  copyHint,
  onCopy,
  onReset,
  resetDisabled,
  onUploadClick,
  expanded,
  onToggleExpand,
  lineCount,
  hasValue,
}: {
  toolbarBtn: string;
  copyHint: string | null;
  onCopy: () => void;
  onReset: () => void;
  resetDisabled: boolean;
  onUploadClick: () => void;
  expanded: boolean;
  onToggleExpand: () => void;
  lineCount: number;
  hasValue: boolean;
}) {
  return (
    <div className="flex flex-wrap items-center gap-2 shrink-0">
      <button type="button" onClick={onCopy} disabled={!hasValue} className={toolbarBtn} title="Copy YAML">
        <Copy className="w-3.5 h-3.5" />
        {copyHint ?? 'Copy'}
      </button>
      <button type="button" onClick={onReset} disabled={resetDisabled} className={toolbarBtn} title="Reset to default">
        <RotateCcw className="w-3.5 h-3.5" />
        Reset
      </button>
      <button type="button" onClick={onUploadClick} className={toolbarBtn} title="Upload YAML file">
        <Upload className="w-3.5 h-3.5" />
        Upload
      </button>
      <button
        type="button"
        onClick={onToggleExpand}
        className={toolbarBtn}
        title={expanded ? 'Collapse editor' : 'Expand editor'}
        aria-pressed={expanded}
      >
        {expanded ? <Minimize2 className="w-3.5 h-3.5" /> : <Maximize2 className="w-3.5 h-3.5" />}
        {expanded ? 'Collapse' : 'Expand'}
      </button>
      <span className="ml-auto text-xs text-slate-500">
        {lineCount} lines
        {expanded ? ' · expanded' : ' · fit to content'}
      </span>
    </div>
  );
}
