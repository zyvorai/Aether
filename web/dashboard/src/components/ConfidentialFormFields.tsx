import type { EditorWorkloadInput } from '../utils/workloadYaml';

export type ConfidentialFormState = Pick<
  EditorWorkloadInput,
  | 'confidentialEnabled'
  | 'confidentialTee'
  | 'confidentialSecurityProfile'
  | 'confidentialKataRuntime'
  | 'attestationRequired'
  | 'imageDigest'
>;

interface ConfidentialFormFieldsProps {
  runtime: string;
  state: ConfidentialFormState;
  onChange: <K extends keyof ConfidentialFormState>(field: K, value: ConfidentialFormState[K]) => void;
}

export default function ConfidentialFormFields({ runtime, state, onChange }: ConfidentialFormFieldsProps) {
  const showKata = runtime === 'kata' || runtime === 'kubernetes';
  const showBlock = runtime === 'kubevirt' || showKata;

  if (!showBlock) {
    return (
      <p className="text-xs text-slate-500">
        Select runtime Kubernetes, Kata, or KubeVirt to configure confidential computing.
      </p>
    );
  }

  return (
    <div className="space-y-3">
      <label className="flex items-center gap-2 text-sm text-slate-300">
        <input
          type="checkbox"
          checked={state.confidentialEnabled ?? false}
          onChange={(e) => onChange('confidentialEnabled', e.target.checked)}
          className="rounded border-slate-600"
        />
        Enable confidential workload (SEV-SNP / TDX)
      </label>
      {state.confidentialEnabled && (
        <div className="grid grid-cols-1 gap-4 md:grid-cols-2 pl-0 sm:pl-2">
          <div>
            <label className="mb-1 block text-xs text-slate-500">TEE</label>
            <select
              value={state.confidentialTee ?? 'sev-snp'}
              onChange={(e) => {
                const tee = e.target.value as NonNullable<ConfidentialFormState['confidentialTee']>;
                onChange('confidentialTee', tee);
                onChange(
                  'confidentialKataRuntime',
                  tee === 'tdx' ? 'kata-clh-tdx' : 'kata-clh-snp',
                );
              }}
              className="w-full rounded-xl border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-100"
            >
              <option value="sev-snp">AMD SEV-SNP</option>
              <option value="tdx">Intel TDX</option>
            </select>
          </div>
          {showKata && (
            <div>
              <label className="mb-1 block text-xs text-slate-500">Security profile</label>
              <select
                value={state.confidentialSecurityProfile ?? ''}
                onChange={(e) =>
                  onChange(
                    'confidentialSecurityProfile',
                    e.target.value as NonNullable<ConfidentialFormState['confidentialSecurityProfile']>,
                  )
                }
                className="w-full rounded-xl border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-100"
              >
                <option value="">Custom (manual runtime class)</option>
                <option value="sandbox">sandbox — no Kata TEE pool</option>
                <option value="standard-confidential">standard-confidential</option>
                <option value="sovereign-high">sovereign-high</option>
              </select>
            </div>
          )}
          {showKata && !state.confidentialSecurityProfile && (
            <div>
              <label className="mb-1 block text-xs text-slate-500">Kata runtime class</label>
              <select
                value={state.confidentialKataRuntime ?? 'kata-clh-snp'}
                onChange={(e) =>
                  onChange(
                    'confidentialKataRuntime',
                    e.target.value as NonNullable<ConfidentialFormState['confidentialKataRuntime']>,
                  )
                }
                className="w-full rounded-xl border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-100"
              >
                <option value="kata-clh-snp">kata-clh-snp (Cloud Hypervisor)</option>
                <option value="kata-clh-tdx">kata-clh-tdx (Cloud Hypervisor)</option>
                <option value="kata-qemu-snp">kata-qemu-snp (legacy QEMU)</option>
                <option value="kata-qemu-tdx">kata-qemu-tdx (legacy QEMU)</option>
              </select>
            </div>
          )}
          <label className="flex items-end gap-2 pb-2 text-sm text-slate-300">
            <input
              type="checkbox"
              checked={state.attestationRequired !== false}
              onChange={(e) => onChange('attestationRequired', e.target.checked)}
              className="rounded border-slate-600"
            />
            Require attestation before deploy
          </label>
          <div className="md:col-span-2">
            <label className="mb-1 block text-xs text-slate-500">Launch digest (from signed catalog)</label>
            <input
              type="text"
              value={state.imageDigest ?? ''}
              onChange={(e) => onChange('imageDigest', e.target.value)}
              placeholder="sha256 launch digest — aether confidential image verify-digest"
              className="w-full rounded-xl border border-slate-700 bg-slate-950 px-3 py-2 font-mono text-sm text-slate-100"
            />
          </div>
        </div>
      )}
    </div>
  );
}

export const defaultConfidentialFormState: ConfidentialFormState = {
  confidentialEnabled: false,
  confidentialTee: 'sev-snp',
  confidentialSecurityProfile: 'standard-confidential',
  confidentialKataRuntime: 'kata-clh-snp',
  attestationRequired: true,
  imageDigest: '',
};
