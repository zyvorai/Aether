// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { forwardRef, type InputHTMLAttributes, type TextareaHTMLAttributes } from 'react';
import { cn } from '../../lib/cn';

export const Input = forwardRef<HTMLInputElement, InputHTMLAttributes<HTMLInputElement>>(
  ({ className, ...props }, ref) => (
    <input ref={ref} className={cn('w-full px-4 py-2.5 bg-surface border border-border rounded-[12px] text-body text-foreground placeholder:text-muted focus-ring', className)} {...props} />
  ),
);
Input.displayName = 'Input';

export const Textarea = forwardRef<HTMLTextAreaElement, TextareaHTMLAttributes<HTMLTextAreaElement>>(
  ({ className, ...props }, ref) => (
    <textarea ref={ref} className={cn('w-full px-4 py-2.5 bg-surface border border-border rounded-[12px] text-body text-foreground placeholder:text-muted focus-ring resize-none', className)} {...props} />
  ),
);
Textarea.displayName = 'Textarea';
