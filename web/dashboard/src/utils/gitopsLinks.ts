/** Guess Aether workload name from a GitOps repo YAML path. */
export function workloadNameFromGitOpsPath(filePath: string): string | null {
  const base = filePath.split('/').pop()?.replace(/\.ya?ml$/i, '').trim();
  if (!base || base.includes('compose') || base === 'kustomization') return null;
  return base;
}
