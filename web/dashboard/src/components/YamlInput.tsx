import { useState, useEffect } from 'react';
import { Send, Loader2 } from 'lucide-react';
import { useTheme } from '../contexts/ThemeContext';

interface YamlInputProps {
  placeholder?: string;
  buttonText: string;
  onSubmit: (yaml: string) => void;
  loading?: boolean;
  initialValue?: string;
  /** `editor` = tall scrollable area for deploy/validate modals; `compact` = smaller inline form. */
  layout?: 'compact' | 'editor';
}

function countLines(text: string): number {
  if (!text) return 1;
  return text.split('\n').length;
}

export default function YamlInput({
  placeholder,
  buttonText,
  onSubmit,
  loading,
  initialValue,
  layout = 'compact',
}: YamlInputProps) {
  const { theme } = useTheme();
  const light = theme === 'light';
  const [value, setValue] = useState(initialValue ?? '');
  const isEditor = layout === 'editor';
  const lineCount = countLines(value);

  useEffect(() => {
    setValue(initialValue ?? '');
  }, [initialValue]);

  function handleSubmit() {
    const trimmed = value.trim();
    if (trimmed && !loading) {
      onSubmit(trimmed);
    }
  }

  const textareaClass = isEditor
    ? `block w-full h-[600px] max-h-[70vh] min-h-[28rem] overflow-y-scroll overflow-x-auto rounded-xl p-4 text-sm font-mono whitespace-pre resize-y leading-6 ${
        light
          ? 'bg-white border border-slate-300 text-slate-800 placeholder-slate-400'
          : 'bg-zinc-950 border border-zinc-700 text-zinc-300 placeholder-zinc-600'
      } focus:outline-none focus:border-aether focus:ring-1 focus:ring-aether/30 transition-colors`
    : `w-full rounded-xl p-4 text-sm font-mono resize-y focus:outline-none focus:border-aether focus:ring-1 focus:ring-aether/30 transition-colors leading-relaxed ${
        light
          ? 'bg-white border border-slate-300 text-slate-800 placeholder-slate-400'
          : 'bg-zinc-950 border border-zinc-700 text-zinc-300 placeholder-zinc-600'
      }`;

  return (
    <div className={isEditor ? 'flex min-h-0 flex-col gap-3' : 'space-y-3'}>
      <textarea
        value={value}
        onChange={(e) => setValue(e.target.value)}
        placeholder={placeholder ?? 'Paste YAML here...'}
        rows={isEditor ? 1 : 10}
        className={textareaClass}
        spellCheck={false}
        aria-label="Workload YAML"
      />
      <div className={`flex items-center justify-between gap-3 ${isEditor ? 'shrink-0' : ''}`}>
        {isEditor ? (
          <span className={`text-xs ${light ? 'text-slate-500' : 'text-zinc-500'}`}>
            {lineCount} lines — scroll inside the box to see the full spec
          </span>
        ) : null}
        <button
          onClick={handleSubmit}
          disabled={!value.trim() || loading}
          className={`flex shrink-0 items-center gap-2 px-5 py-2.5 bg-aether hover:bg-aether-light disabled:opacity-40 disabled:cursor-not-allowed text-white text-sm font-medium rounded-lg transition-colors focus:outline-none focus:ring-2 focus:ring-aether/50 ${isEditor ? '' : 'ml-auto w-full sm:w-auto justify-center'}`}
        >
          {loading ? (
            <Loader2 className="w-4 h-4 animate-spin" />
          ) : (
            <Send className="w-4 h-4" />
          )}
          {buttonText}
        </button>
      </div>
    </div>
  );
}
