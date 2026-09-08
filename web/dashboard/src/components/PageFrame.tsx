// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import type { ReactNode } from 'react';
import PageLoading from './PageLoading';
import PageLoadError from './PageLoadError';

interface PageFrameProps {
  loading: boolean;
  error: boolean;
  hasData: boolean;
  onRetry: () => void;
  errorTitle?: string;
  errorDescription?: string;
  loadingRows?: number;
  toolbar?: ReactNode;
  contextBanner?: ReactNode;
  empty?: ReactNode;
  isEmpty?: boolean;
  children: ReactNode;
}

export default function PageFrame({
  loading,
  error,
  hasData,
  onRetry,
  errorTitle = 'Data unavailable',
  errorDescription,
  loadingRows = 4,
  toolbar,
  contextBanner,
  empty,
  isEmpty = false,
  children,
}: PageFrameProps) {
  if (loading && !hasData && !error) {
    return <PageLoading rows={loadingRows} />;
  }

  if (error && !hasData) {
    return (
      <PageLoadError
        title={errorTitle}
        description={errorDescription}
        onRetry={onRetry}
      />
    );
  }

  return (
    <div>
      {contextBanner}
      {toolbar}
      {isEmpty && empty ? empty : children}
    </div>
  );
}
