import { Hexagon } from 'lucide-react';

export default function Footer() {
  return (
    <footer className="bg-zinc-950 border-t border-zinc-800 mt-16">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
        <div className="grid grid-cols-2 md:grid-cols-4 gap-8">
          {/* About Aether */}
          <div>
            <div className="flex items-center gap-2 mb-4">
              <Hexagon className="w-5 h-5 text-aether" />
              <span className="font-semibold text-white">Aether</span>
            </div>
            <p className="text-sm text-zinc-400 leading-relaxed">
              Universal Runtime Control Plane. Deploy workloads to any runtime from a single YAML specification.
            </p>
          </div>

          {/* Runtimes */}
          <div>
            <h4 className="text-sm font-semibold text-zinc-200 uppercase tracking-wider mb-4">
              Runtimes
            </h4>
            <ul className="space-y-2 text-sm text-zinc-400">
              <li>Podman</li>
              <li>Kubernetes</li>
              <li>KubeVirt</li>
              <li>Metal3</li>
              <li>Docker</li>
            </ul>
          </div>

          {/* Features */}
          <div>
            <h4 className="text-sm font-semibold text-zinc-200 uppercase tracking-wider mb-4">
              Features
            </h4>
            <ul className="space-y-2 text-sm text-zinc-400">
              <li>AI Engine</li>
              <li>Cost Estimation</li>
              <li>Drift Detection</li>
              <li>Policy Enforcement</li>
              <li>Secrets Management</li>
            </ul>
          </div>

          {/* Support */}
          <div>
            <h4 className="text-sm font-semibold text-zinc-200 uppercase tracking-wider mb-4">
              Support
            </h4>
            <ul className="space-y-2 text-sm text-zinc-400">
              <li>Documentation</li>
              <li>API Reference</li>
              <li>Community</li>
              <li>GitHub</li>
              <li>Changelog</li>
            </ul>
          </div>
        </div>
      </div>

      {/* Bottom bar */}
      <div className="border-t border-zinc-800">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-4 flex flex-col sm:flex-row items-center justify-between gap-2">
          <p className="text-xs text-zinc-500">
            &copy; {new Date().getFullYear()} Aether Project. All rights reserved.
          </p>
          <div className="flex items-center gap-4">
            <span className="text-xs text-zinc-500 hover:text-zinc-300 cursor-pointer transition-colors">
              Privacy
            </span>
            <span className="text-xs text-zinc-500 hover:text-zinc-300 cursor-pointer transition-colors">
              Terms
            </span>
          </div>
        </div>
      </div>
    </footer>
  );
}
