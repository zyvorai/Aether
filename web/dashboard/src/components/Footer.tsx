import { Hexagon } from 'lucide-react';

export default function Footer() {
  return (
    <footer className="mt-16 border-t border-slate-800/70 bg-slate-950/70 backdrop-blur-sm">
      <div className="dash-content py-12">
        <div className="grid gap-8 lg:grid-cols-[1.3fr_repeat(3,1fr)]">
          <div className="surface-panel rounded-[24px] p-6">
            <div className="flex items-center gap-3 mb-4">
              <div className="flex h-10 w-10 items-center justify-center rounded-xl border border-aether/20 bg-aether/10">
                <Hexagon className="w-5 h-5 text-aether" />
              </div>
              <div>
                <div className="font-semibold text-white">Aether</div>
                <div className="text-xs uppercase tracking-[0.18em] text-slate-500">Universal Runtime Control Plane</div>
              </div>
            </div>
            <p className="text-sm text-slate-400 leading-relaxed">
              A single operator surface for Kubernetes, KubeVirt, Metal3, and container runtimes, redesigned around a sharper control-room UX.
            </p>
          </div>

          <div>
            <h4 className="text-sm font-semibold text-slate-200 uppercase tracking-wider mb-4">
              Runtimes
            </h4>
            <ul className="space-y-2 text-sm text-slate-400">
              <li>Podman</li>
              <li>Kubernetes</li>
              <li>KubeVirt</li>
              <li>Metal3</li>
              <li>Docker</li>
            </ul>
          </div>

          <div>
            <h4 className="text-sm font-semibold text-slate-200 uppercase tracking-wider mb-4">
              Surfaces
            </h4>
            <ul className="space-y-2 text-sm text-slate-400">
              <li>Cluster Browser</li>
              <li>Workload Inventory</li>
              <li>Exec and Port Forward</li>
              <li>Helm and Rollouts</li>
              <li>Audit and Metrics</li>
            </ul>
          </div>

          <div>
            <h4 className="text-sm font-semibold text-slate-200 uppercase tracking-wider mb-4">
              Access
            </h4>
            <ul className="space-y-2 text-sm text-slate-400">
              <li>Bearer token or RBAC key (set in dashboard sign-in)</li>
              <li>Legacy env: <code className="text-slate-300">AETHER_API_KEY</code></li>
              <li>Documentation</li>
              <li>API Reference</li>
              <li>GitHub</li>
              <li>Live Cluster Ops</li>
            </ul>
          </div>
        </div>
      </div>

      <div className="border-t border-slate-800/70">
        <div className="dash-content py-4 flex flex-col sm:flex-row sm:flex-wrap items-center justify-center sm:justify-between gap-x-4 gap-y-2 text-center sm:text-left">
          <p className="text-xs text-slate-500 min-w-0 max-w-full leading-relaxed">
            &copy; {new Date().getFullYear()} Aether Project. Reframed with a Machina-style shell.
          </p>
          <div className="flex flex-wrap items-center justify-center sm:justify-end gap-x-4 gap-y-1 shrink-0">
            <span className="text-xs text-slate-500">Bearer, RBAC key, or OIDC when enabled</span>
          </div>
        </div>
      </div>
    </footer>
  );
}
