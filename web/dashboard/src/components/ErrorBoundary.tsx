// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { Component, type ErrorInfo, type ReactNode } from 'react';
import { AlertTriangle, Copy, RefreshCw } from 'lucide-react';
import { copyToClipboard } from '../utils/clipboard';

interface Props {
  children: ReactNode;
  /** A new value clears the error, so one broken page does not block the others. */
  resetKey?: string;
}

interface State {
  error: Error | null;
  resetKey?: string;
}

export default class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { error: null, resetKey: props.resetKey };
  }

  static getDerivedStateFromError(error: Error): Partial<State> {
    return { error };
  }

  static getDerivedStateFromProps(props: Props, state: State): Partial<State> | null {
    return props.resetKey === state.resetKey ? null : { error: null, resetKey: props.resetKey };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error('ErrorBoundary caught:', error, info.componentStack);
  }

  render() {
    const { error } = this.state;
    if (!error) return this.props.children;
    const message = error.message || 'An unexpected error occurred while rendering this page.';
    return (
      <div className="flex min-h-[300px] items-center justify-center p-8" role="alert">
        <div className="glass w-full max-w-lg rounded-[var(--radius-xl)] p-8 text-center shadow-lg">
          <div className="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-full bg-danger/10">
            <AlertTriangle className="h-6 w-6 text-danger" aria-hidden />
          </div>
          <h2 className="mb-2 text-lg font-semibold text-foreground">Something went wrong</h2>
          <p className="mb-2 text-sm text-muted">This page could not be drawn. The rest of the dashboard still works.</p>
          <code className="state-detail mb-6">{message}</code>
          <div className="flex flex-wrap items-center justify-center gap-2">
            <button type="button" onClick={() => this.setState({ error: null })} className="btn-secondary">
              Try Again
            </button>
            <button
              type="button"
              onClick={() => void copyToClipboard(`${message}\n${window.location.href}\n${error.stack ?? ''}`)}
              className="btn-secondary gap-2"
            >
              <Copy className="h-4 w-4" aria-hidden />
              Copy details
            </button>
            <button type="button" onClick={() => window.location.reload()} className="btn-primary">
              <RefreshCw className="h-4 w-4" aria-hidden />
              Reload Page
            </button>
          </div>
        </div>
      </div>
    );
  }
}
