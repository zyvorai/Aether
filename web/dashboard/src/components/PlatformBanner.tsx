import { AlertTriangle } from 'lucide-react';
import { useServerCapabilities } from '../contexts/ServerCapabilitiesContext';

export default function PlatformBanner() {
  const { capabilities } = useServerCapabilities();
  const rec = capabilities?.platform?.haRecommendation;
  if (!rec) return null;

  return (
    <div className="mb-6 flex gap-3 rounded-xl border border-amber-700/40 bg-amber-950/30 px-4 py-3 text-sm text-amber-100">
      <AlertTriangle className="shrink-0 text-amber-400 mt-0.5" size={18} aria-hidden />
      <p>{rec}</p>
    </div>
  );
}
