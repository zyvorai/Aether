interface CodeBlockProps {
  children: string;
  title?: string;
}

export default function CodeBlock({ children, title }: CodeBlockProps) {
  return (
    <div className="rounded-xl overflow-hidden border border-zinc-700">
      {title && (
        <div className="bg-zinc-800 px-4 py-2 border-b border-zinc-700">
          <span className="text-xs font-medium text-zinc-400 uppercase tracking-wider">
            {title}
          </span>
        </div>
      )}
      <pre className="bg-zinc-950 p-4 overflow-x-auto text-sm text-zinc-300 font-mono leading-relaxed">
        <code>{children}</code>
      </pre>
    </div>
  );
}
