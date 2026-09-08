// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

/** Build a Hubble UI deep link for a pod (matches backend hubble_workload_url). */
export function hubbleWorkloadUrl(base: string, namespace: string, pod: string): string {
  const trimmed = base.replace(/\/+$/, '');
  const params = new URLSearchParams({ namespace, pod });
  return `${trimmed}/?${params.toString()}`;
}

/** Namespace-level Hubble view when pod name is unknown. */
export function hubbleNamespaceUrl(base: string, namespace: string): string {
  const trimmed = base.replace(/\/+$/, '');
  const params = new URLSearchParams({ namespace });
  return `${trimmed}/?${params.toString()}`;
}
