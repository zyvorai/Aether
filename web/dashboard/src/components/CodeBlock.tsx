// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

interface CodeBlockProps {
  children: string;
  title?: string;
}

export default function CodeBlock({ children, title }: CodeBlockProps) {
  return (
    <div className="glass-code-block">
      {title && (
        <div className="glass-code-block-header">
          <span className="text-xs font-medium text-muted uppercase tracking-wider">
            {title}
          </span>
        </div>
      )}
      <pre className="glass-code-block-body">
        <code>{children}</code>
      </pre>
    </div>
  );
}
