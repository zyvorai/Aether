// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useEffect, useState } from 'react';
import { apiFetchSettled } from '../utils/api';
import type { EditorWorkloadInput } from '../utils/workloadYaml';
import type { SecurityProfileEntry } from '../types/api';

export type ConfidentialFormState = Pick<
  EditorWorkloadInput,
  | 'confidentialEnabled'
  | 'confidentialTee'
  | 'confidentialSecurityProfile'
  | 'confidentialKataRuntime'
  | 'attestationRequired'
  | 'attestationPolicy'
  | 'confidentialRegionLock'
  | 'confidentialSecretNames'
  | 'confidentialVtpm'
  | 'confidentialEncryptedState'
  | 'confidentialDebugAllowed'
  | 'imageDigest'
>;

interface ConfidentialFormFieldsProps {
  runtime: string;
  state: ConfidentialFormState;
  onChange: <K extends keyof ConfidentialFormState>(field: K, value: ConfidentialFormState[K]) => void;
}

const FALLBACK_PROFILES: SecurityProfileEntry[] = [
  { id: 'sandbox', label: 'sandbox', description: 'Non-confidential sandbox pool' },
  { id: 'standard-confidential', label: 'standard-confidential', description: 'Standard confidential Kata pool' },
  { id: 'sovereign-high', label: 'sovereign-high', description: 'High-assurance sovereign pool' },
];

export default function ConfidentialFormFields({ runtime, state, onChange }: ConfidentialFormFieldsProps) {
  const [profiles, setProfiles] = useState<SecurityProfileEntry[]>(FALLBACK_PROFILES);

  useEffect(() => {
    void (async () => {
      const res = await apiFetchSettled<{ profiles?: SecurityProfileEntry[] }>('/confidential/security-profiles');
      if (res.ok && res.data.profiles && res.data.profiles.length > 0) {
        setProfiles(res.data.profiles);
      }
    })();
  }, []);

  const showKata = runtime === 'kata' || runtime === 'kubernetes';
  const showBlock = runtime === 'kubevirt' || showKata;

  if (!showBlock) {
    return (
      <p className="text-xs text-subtle">
        Select runtime Kubernetes, Kata, or KubeVirt to configure confidential computing.
      </p>
    );
  }

  return (
    <div className="space-y-3">
      <label className="flex items-center gap-2 text-sm text-muted">
        <input
          type="checkbox"
          checked={state.confidentialEnabled ?? false}
          onChange={(e) => onChange('confidentialEnabled', e.target.checked)}
          className="rounded glass-divider"
        />
        Enable confidential workload (SEV-SNP / TDX)
      </label>
      {state.confidentialEnabled && (
        <div className="grid grid-cols-1 gap-4 md:grid-cols-2 pl-0 sm:pl-2">
          <div>
            <label className="mb-1 block text-xs text-subtle">TEE</label>
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
              className="glass-input"
            >
              <option value="sev-snp">AMD SEV-SNP</option>
              <option value="tdx">Intel TDX</option>
            </select>
          </div>
          {showKata && (
            <div>
              <label className="mb-1 block text-xs text-subtle">Security profile</label>
              <select
                value={state.confidentialSecurityProfile ?? ''}
                onChange={(e) => onChange('confidentialSecurityProfile', e.target.value)}
                className="glass-input"
              >
                <option value="">Custom (manual runtime class)</option>
                {profiles.map((p) => (
                  <option key={p.id} value={p.id} title={p.description}>
                    {p.label}
                  </option>
                ))}
              </select>
            </div>
          )}
          {showKata && !state.confidentialSecurityProfile && (
            <div>
              <label className="mb-1 block text-xs text-subtle">Kata runtime class</label>
              <select
                value={state.confidentialKataRuntime ?? 'kata-clh-snp'}
                onChange={(e) =>
                  onChange(
                    'confidentialKataRuntime',
                    e.target.value as NonNullable<ConfidentialFormState['confidentialKataRuntime']>,
                  )
                }
                className="glass-input"
              >
                <option value="kata-clh-snp">kata-clh-snp (Cloud Hypervisor)</option>
                <option value="kata-clh-tdx">kata-clh-tdx (Cloud Hypervisor)</option>
                <option value="kata-qemu-snp">kata-qemu-snp (legacy QEMU)</option>
                <option value="kata-qemu-tdx">kata-qemu-tdx (legacy QEMU)</option>
              </select>
            </div>
          )}
          <div>
            <label className="mb-1 block text-xs text-subtle">Attestation policy</label>
            <select
              value={state.attestationPolicy ?? 'strict'}
              onChange={(e) =>
                onChange('attestationPolicy', e.target.value as NonNullable<ConfidentialFormState['attestationPolicy']>)
              }
              className="glass-input"
            >
              <option value="strict">strict — digest required</option>
              <option value="standard">standard — baseline measurements</option>
            </select>
          </div>
          <div>
            <label className="mb-1 block text-xs text-subtle">Region lock</label>
            <input
              type="text"
              value={state.confidentialRegionLock ?? ''}
              onChange={(e) => onChange('confidentialRegionLock', e.target.value)}
              placeholder="e.g. us-east-1 or sovereign-zone-a"
              className="glass-input"
            />
          </div>
          <label className="flex items-end gap-2 pb-2 text-sm text-muted">
            <input
              type="checkbox"
              checked={state.attestationRequired !== false}
              onChange={(e) => onChange('attestationRequired', e.target.checked)}
              className="rounded glass-divider"
            />
            Require attestation before deploy
          </label>
          <label className="flex items-end gap-2 pb-2 text-sm text-muted">
            <input
              type="checkbox"
              checked={state.confidentialVtpm !== false}
              onChange={(e) => onChange('confidentialVtpm', e.target.checked)}
              className="rounded glass-divider"
            />
            vTPM
          </label>
          <label className="flex items-end gap-2 pb-2 text-sm text-muted">
            <input
              type="checkbox"
              checked={state.confidentialEncryptedState !== false}
              onChange={(e) => onChange('confidentialEncryptedState', e.target.checked)}
              className="rounded glass-divider"
            />
            Encrypted state
          </label>
          <label className="flex items-end gap-2 pb-2 text-sm text-muted">
            <input
              type="checkbox"
              checked={state.confidentialDebugAllowed === true}
              onChange={(e) => onChange('confidentialDebugAllowed', e.target.checked)}
              className="rounded glass-divider"
            />
            Allow debug
          </label>
          <div className="md:col-span-2">
            <label className="mb-1 block text-xs text-subtle">Attest-gated secret names (comma or newline)</label>
            <textarea
              value={state.confidentialSecretNames ?? ''}
              onChange={(e) => onChange('confidentialSecretNames', e.target.value)}
              rows={2}
              placeholder="db-credentials, api-key"
              className="glass-input"
            />
          </div>
          <div className="md:col-span-2">
            <label className="mb-1 block text-xs text-subtle">Launch digest (from signed catalog)</label>
            <input
              type="text"
              value={state.imageDigest ?? ''}
              onChange={(e) => onChange('imageDigest', e.target.value)}
              placeholder="sha256 launch digest — aether confidential image verify-digest"
              className="glass-input font-mono"
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
  attestationPolicy: 'strict',
  confidentialRegionLock: '',
  confidentialSecretNames: '',
  confidentialVtpm: true,
  confidentialEncryptedState: true,
  confidentialDebugAllowed: false,
  imageDigest: '',
};
