// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

/** Extract workload name from generated template JSON or YAML-shaped spec. */
export function workloadNameFromSpec(spec: Record<string, unknown> | null | undefined): string | null {
  if (!spec) return null;
  const meta = spec.metadata as { name?: string } | undefined;
  if (meta?.name && typeof meta.name === 'string') return meta.name;
  if (typeof spec.name === 'string') return spec.name;
  return null;
}
