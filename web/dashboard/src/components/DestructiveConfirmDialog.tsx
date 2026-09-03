// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useState } from 'react';
import Modal from './Modal';

interface DestructiveConfirmDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onConfirm: () => void;
  title: string;
  resourceName: string;
  description?: string;
  confirmLabel?: string;
}

export default function DestructiveConfirmDialog({
  isOpen,
  onClose,
  onConfirm,
  title,
  resourceName,
  description,
  confirmLabel = 'Delete',
}: DestructiveConfirmDialogProps) {
  const [typed, setTyped] = useState('');
  const matches = typed === resourceName;

  const close = () => {
    setTyped('');
    onClose();
  };

  const commit = () => {
    if (!matches) return;
    onConfirm();
    setTyped('');
  };

  return (
    <Modal isOpen={isOpen} onClose={close} title={title}>
      <p className="mb-4 text-sm text-ink-2">
        {description ?? 'This action cannot be undone.'} Type{' '}
        <span className="font-mono text-ink">{resourceName}</span> to confirm.
      </p>
      <input
        value={typed}
        autoFocus
        placeholder={resourceName}
        onChange={(e) => setTyped(e.target.value)}
        onKeyDown={(e) => e.key === 'Enter' && commit()}
        className="w-full rounded-lg border border-rule-strong bg-surface px-3 py-2 font-mono text-[12.5px] text-ink outline-none focus-visible:ring-2 focus-visible:ring-red-500/40"
      />
      <div className="mt-5 flex justify-end gap-2.5">
        <button
          type="button"
          onClick={close}
          className="rounded-full border border-rule-strong px-4 py-1.5 text-[13px] text-ink-2 hover:bg-hover"
        >
          Cancel
        </button>
        <button
          type="button"
          onClick={commit}
          disabled={!matches}
          className="rounded-full bg-red-600 px-4 py-1.5 text-[13px] font-medium text-white disabled:cursor-not-allowed disabled:opacity-40 hover:bg-red-500"
        >
          {confirmLabel}
        </button>
      </div>
    </Modal>
  );
}
