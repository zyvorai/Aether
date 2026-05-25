import { Eye } from 'lucide-react';
import { useAuth } from '../contexts/AuthContext';
import { useTheme } from '../contexts/ThemeContext';

export default function ViewerBanner() {
  const { isViewer } = useAuth();
  const { theme } = useTheme();
  const light = theme === 'light';

  if (!isViewer) return null;

  return (
    <div
      role="status"
      className={`border-b px-4 py-2 text-sm ${
        light
          ? 'border-slate-200 bg-slate-100 text-slate-700'
          : 'border-slate-700/50 bg-slate-900/60 text-slate-300'
      }`}
    >
      <div className="dash-content flex items-center gap-2">
        <Eye className="h-4 w-4 shrink-0 text-aether" aria-hidden />
        <span>Read-only session — deploy and mutation actions are disabled for the Viewer role.</span>
      </div>
    </div>
  );
}
