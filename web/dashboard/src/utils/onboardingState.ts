// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import type { WorkloadResponse } from '../types/api';

const VALIDATED_KEY = 'aether_onboarding_validated';
const DEPLOYED_KEY = 'aether_onboarding_deployed';
const HEALTH_KEY = 'aether_onboarding_health';

function setKey(key: string): void {
  try {
    localStorage.setItem(key, '1');
  } catch {
    /* ignore */
  }
}

function hasKey(key: string): boolean {
  try {
    return localStorage.getItem(key) === '1';
  } catch {
    return false;
  }
}

export function markSpecValidated(): void {
  setKey(VALIDATED_KEY);
  window.dispatchEvent(new Event('aether:spec-validated'));
}

export function hasValidatedSpec(): boolean {
  return hasKey(VALIDATED_KEY);
}

export function markFirstDeploy(): void {
  setKey(DEPLOYED_KEY);
  window.dispatchEvent(new Event('aether:first-deploy'));
}

export function hasDeployedWorkload(): boolean {
  return hasKey(DEPLOYED_KEY);
}

export function markHealthReviewed(): void {
  setKey(HEALTH_KEY);
  window.dispatchEvent(new Event('aether:health-reviewed'));
}

export function hasReviewedHealth(): boolean {
  return hasKey(HEALTH_KEY);
}

export function syncDeployFromWorkloads(workloads: WorkloadResponse[]): void {
  const count = workloads.filter((w) => (w.source ?? 'aether') === 'aether').length;
  if (count > 0 && !hasDeployedWorkload()) {
    markFirstDeploy();
  }
}

export function syncHealthFromSummary(healthy: number, degraded: number, unhealthy: number): void {
  if ((healthy + degraded + unhealthy) > 0 && !hasReviewedHealth()) {
    markHealthReviewed();
  }
}
