import { useEffect, useState, type ReactNode } from 'react';
import { X } from 'lucide-react';
import { cn } from '../../lib/cn';
import { Button } from './Button';
import { SubsectionTitle } from './Typography';

export function Modal({ open, onClose, title, children, className }: {
  open: boolean; onClose: () => void; title: string; children: ReactNode; className?: string;
}) {
  const [rendered, setRendered] = useState(open);
  useEffect(() => {
    if (open) { setRendered(true); return; }
    const timer = window.setTimeout(() => setRendered(false), 150);
    return () => window.clearTimeout(timer);
  }, [open]);
  useEffect(() => {
    if (!open) return;
    const onKey = (event: KeyboardEvent) => event.key === 'Escape' && onClose();
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [open, onClose]);
  if (!rendered) return null;
  return (
    <div className={cn('glass-modal-backdrop fixed inset-0 z-50 flex items-center justify-center p-6', open ? 'animate-fade-in' : 'animate-fade-out')} role="dialog" aria-modal="true" aria-labelledby="modal-title" onClick={onClose}>
      <div className={cn('bg-surface-elevated rounded-[var(--radius-lg)] w-full max-w-md shadow-[var(--shadow-card)] border border-border', open ? 'animate-glass-in' : 'animate-glass-out', className)} onClick={(event) => event.stopPropagation()}>
        <div className="flex items-center justify-between px-6 py-4 border-b border-border">
          <SubsectionTitle as="h2" id="modal-title">{title}</SubsectionTitle>
          <Button variant="ghost" size="sm" onClick={onClose} aria-label="Close"><X className="w-4 h-4" /></Button>
        </div>
        <div className="p-6">{children}</div>
      </div>
    </div>
  );
}
