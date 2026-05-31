// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

interface CodeBlockProps {
  children: string;
  title?: string;
}

export default function CodeBlock({ children, title }: CodeBlockProps) {
  return (
    <div className="glass-panel-card overflow-hidden rounded-xl border border-slate-800/60">
      {title && (
        <div className="bg-slate-800/60 px-4 py-2 border-b border-slate-800/60">
          <span className="text-xs font-medium text-slate-400 uppercase tracking-wider">
            {title}
          </span>
        </div>
      )}
      <pre className="bg-[#0B0E14] p-4 overflow-x-auto text-sm text-slate-300 font-mono leading-relaxed">
        <code>{children}</code>
      </pre>
    </div>
  );
}
