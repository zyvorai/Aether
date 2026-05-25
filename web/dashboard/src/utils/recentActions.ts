export interface RecentAction {
  id: string;
  label: string;
  searchText: string;
}

const STORAGE_KEY = 'aether_recent_actions';
const MAX_RECENT = 5;

export function pushRecentAction(action: RecentAction): void {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    const parsed = raw ? (JSON.parse(raw) as unknown) : [];
    const existing = Array.isArray(parsed)
      ? parsed.filter((item): item is RecentAction => {
          if (!item || typeof item !== 'object') return false;
          const row = item as RecentAction;
          return typeof row.id === 'string' && typeof row.label === 'string';
        })
      : [];
    const next = [action, ...existing.filter((a) => a.id !== action.id)].slice(0, MAX_RECENT);
    localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
  } catch {
    /* ignore */
  }
}

export function getRecentActions(): RecentAction[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    const parsed = raw ? (JSON.parse(raw) as unknown) : [];
    if (!Array.isArray(parsed)) return [];
    return parsed.filter((item): item is RecentAction => {
      if (!item || typeof item !== 'object') return false;
      const row = item as RecentAction;
      return typeof row.id === 'string' && typeof row.label === 'string';
    });
  } catch {
    return [];
  }
}
