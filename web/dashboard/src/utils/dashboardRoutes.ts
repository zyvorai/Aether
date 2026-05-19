import type { AppView } from '../types/api';

/** URL path for each dashboard screen (browser history, refresh-safe). */
export const VIEW_TO_PATH: Record<AppView, string> = {
  overview: '/',
  workloads: '/workloads',
  clusters: '/clusters',
  compose: '/compose',
  ai: '/ai',
  cost: '/cost',
  affinity: '/affinity',
  drift: '/drift',
  policy: '/policy',
  scheduler: '/scheduler',
  health: '/health',
  events: '/events',
  sla: '/sla',
  deps: '/deps',
  envs: '/envs',
  secrets: '/secrets',
  backups: '/backups',
  templates: '/templates',
  plugins: '/plugins',
  rbac: '/rbac',
  audit: '/audit',
  metrics: '/metrics',
  gitops: '/gitops',
};

const PATH_TO_VIEW = new Map<string, AppView>();
for (const [view, path] of Object.entries(VIEW_TO_PATH) as [AppView, string][]) {
  PATH_TO_VIEW.set(path, view);
}

export function viewToPath(view: AppView): string {
  return VIEW_TO_PATH[view];
}

/** Resolve `location.pathname` to a view, or `null` if the path is not a dashboard route. */
export function pathToView(pathname: string): AppView | null {
  const normalized = pathname.replace(/\/+$/, '') || '/';
  return PATH_TO_VIEW.get(normalized) ?? null;
}
