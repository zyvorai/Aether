import { useState, useEffect } from 'react';
import { Send, Loader2 } from 'lucide-react';
import { useTheme } from '../contexts/ThemeContext';

interface YamlInputProps {
  placeholder?: string;
  buttonText: string;
  onSubmit: (yaml: string) => void;
  loading?: boolean;
  initialValue?: string;
}

export default function YamlInput({ placeholder, buttonText, onSubmit, loading, initialValue }: YamlInputProps) {
  const { theme } = useTheme();
  const light = theme === 'light';
  const [value, setValue] = useState(initialValue ?? '');

  useEffect(() => {
    setValue(initialValue ?? '');
  }, [initialValue]);

  function handleSubmit() {
    const trimmed = value.trim();
    if (trimmed && !loading) {
      onSubmit(trimmed);
    }
  }

  return (
    <div className="space-y-3">
      <textarea
        value={value}
        onChange={(e) => setValue(e.target.value)}
        placeholder={placeholder ?? 'Paste YAML here...'}
        rows={10}
        className={`w-full rounded-xl p-4 text-sm font-mono resize-y focus:outline-none focus:border-aether focus:ring-1 focus:ring-aether/30 transition-colors leading-relaxed ${
          light
            ? 'bg-white border border-slate-300 text-slate-800 placeholder-slate-400'
            : 'bg-zinc-950 border border-zinc-700 text-zinc-300 placeholder-zinc-600'
        }`}
        spellCheck={false}
      />
      <div className="flex justify-end">
        <button
          onClick={handleSubmit}
          disabled={!value.trim() || loading}
          className="flex items-center gap-2 px-5 py-2.5 bg-aether hover:bg-aether-light disabled:opacity-40 disabled:cursor-not-allowed text-white text-sm font-medium rounded-lg transition-colors focus:outline-none focus:ring-2 focus:ring-aether/50"
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
