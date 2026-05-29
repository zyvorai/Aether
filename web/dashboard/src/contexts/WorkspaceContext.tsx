// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { createContext, useContext, useMemo, useState, type ReactNode } from 'react';

interface WorkspaceContextValue {
  workspace: string;
  setWorkspace: (ns: string) => void;
}

const WorkspaceContext = createContext<WorkspaceContextValue>({
  workspace: 'all',
  setWorkspace: () => {},
});

export function WorkspaceProvider({ children }: { children: ReactNode }) {
  const [workspace, setWorkspace] = useState('all');
  const value = useMemo(() => ({ workspace, setWorkspace }), [workspace]);
  return <WorkspaceContext.Provider value={value}>{children}</WorkspaceContext.Provider>;
}

export function useWorkspace() {
  return useContext(WorkspaceContext);
}
