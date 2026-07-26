// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

/** Pick a Running pod when available, otherwise the first related pod. */
export function pickExecPodName(
  kind: string | undefined,
  resourceName: string,
  pods: Array<{ name: string; phase: string }>,
): string {
  if (kind === 'Pod') return resourceName;
  const running = pods.find((pod) => pod.phase === 'Running');
  return running?.name ?? pods[0]?.name ?? '';
}
