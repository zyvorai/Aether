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
import { useTheme } from '../contexts/ThemeContext';
import { countYamlLines, yamlEditorHeightPx } from '../utils/editorHeight';
import YamlCodeEditor from './YamlCodeEditor';

interface YamlInputProps {
  placeholder?: string;
  buttonText: string;
  onSubmit: (yaml: string) => void;
  loading?: boolean;
  initialValue?: string;
  resetValue?: string;
  layout?: 'compact' | 'editor';
  submitDisabled?: boolean;
  editorLoading?: boolean;
  onValidate?: (yaml: string) => void;
  validateLoading?: boolean;
  showValidateButton?: boolean;
  footer?: ReactNode;
}

export default function YamlInput({
  placeholder,
  buttonText,
  onSubmit,
  loading,
  initialValue,
  resetValue,
  layout = 'compact',
  submitDisabled,
  editorLoading,
  onValidate,
  validateLoading,
  showValidateButton,
  footer,
}: YamlInputProps) {
  const { theme } = useTheme();
  const light = theme === 'light';
  const [value, setValue] = useState(initialValue ?? '');
  const [expanded, setExpanded] = useState(false);
  const [copyHint, setCopyHint] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const isEditor = layout === 'editor';
  const lineCount = countYamlLines(value);
  const editorHeight = yamlEditorHeightPx(lineCount, expanded);
  const resetTarget = resetValue ?? initialValue ?? '';

  useEffect(() => {
    setValue(initialValue ?? '');
  }, [initialValue]);

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

  const toolbarBtn = `inline-flex items-center gap-1.5 rounded-lg border px-2.5 py-1.5 text-xs font-medium transition-colors focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-aether/40 disabled:opacity-40 disabled:cursor-not-allowed ${
    light
      ? 'border-slate-200 bg-slate-50 text-slate-600 hover:bg-slate-100 hover:text-slate-900'
      : 'border-zinc-700 bg-zinc-900/60 text-zinc-400 hover:bg-zinc-800 hover:text-zinc-200'
  }`;

  const textareaClass = `w-full rounded-xl p-4 text-sm font-mono resize-y focus:outline-none focus:border-aether focus:ring-1 focus:ring-aether/30 transition-colors leading-relaxed ${
    light
      ? 'bg-white border border-slate-300 text-slate-800 placeholder-slate-400'
      : 'bg-zinc-950 border border-zinc-700 text-zinc-300 placeholder-zinc-600'
  }`;

  return (
    <div className={isEditor ? 'flex min-h-0 flex-1 flex-col gap-3 overflow-hidden' : 'space-y-3'}>
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
            light={light}
            hasValue={!!value}
          />
          <input
            ref={fileInputRef}
            type="file"
            accept=".yaml,.yml,text/yaml"
            className="hidden"
            onChange={handleFileUpload}
          />

          <div className="relative min-h-0 flex-1">
            {editorLoading ? (
              <div
                className={`absolute inset-0 z-10 flex items-center justify-center rounded-xl border ${
                  light ? 'border-slate-200 bg-white/90' : 'border-zinc-700 bg-zinc-950/90'
                }`}
              >
                <Loader2 className="w-6 h-6 animate-spin text-aether" />
                <span className={`ml-2 text-sm ${light ? 'text-slate-600' : 'text-zinc-400'}`}>
                  Loading template…
                </span>
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

      <div className={`flex flex-wrap items-center justify-end gap-2 ${isEditor ? 'shrink-0 pt-1' : ''}`}>
        {showValidateButton && onValidate ? (
          <button
            type="button"
            onClick={handleValidateClick}
            disabled={!value.trim() || validateLoading || editorLoading}
            className={`flex items-center gap-2 px-4 py-2.5 border text-sm font-medium rounded-lg transition-colors focus:outline-none focus:ring-2 focus:ring-aether/50 disabled:opacity-40 disabled:cursor-not-allowed ${
              light
                ? 'border-slate-300 bg-white text-slate-700 hover:bg-slate-50'
                : 'border-zinc-600 bg-zinc-900 text-zinc-300 hover:bg-zinc-800'
            }`}
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
          onClick={handleSubmit}
          disabled={!value.trim() || loading || submitDisabled || editorLoading}
          className={`flex shrink-0 items-center gap-2 px-5 py-2.5 bg-aether hover:bg-aether-light disabled:opacity-40 disabled:cursor-not-allowed text-white text-sm font-medium rounded-lg transition-colors focus:outline-none focus:ring-2 focus:ring-aether/50 ${isEditor ? '' : 'w-full sm:w-auto justify-center'}`}
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
  light,
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
  light: boolean;
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
      <span className={`ml-auto text-xs ${light ? 'text-slate-500' : 'text-zinc-500'}`}>
        {lineCount} lines
        {expanded ? ' · expanded' : ' · fit to content'}
      </span>
    </div>
  );
}
