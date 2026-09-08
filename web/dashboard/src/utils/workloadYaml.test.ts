// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import { buildEditorWorkloadYaml } from './workloadYaml';

describe('buildEditorWorkloadYaml kubernetes advanced fields', () => {
  it('includes kubernetes block with gateway, vpa, and pdb', () => {
    const yaml = buildEditorWorkloadYaml({
      name: 'api',
      image: 'nginx:latest',
      runtime: 'kubernetes',
      replicas: 2,
      cpu: '500m',
      memory: '512Mi',
      intent: 'balanced',
      healthCheck: false,
      k8sNamespace: 'prod',
      k8sServiceAccount: 'api-sa',
      k8sNodeSelector: 'nodepool=apps',
      scalingEnabled: false,
      scalingMin: 1,
      scalingMax: 3,
      ingressEnabled: true,
      ingressHost: 'api.example.com',
      ingressPath: '/',
      networkDenyAllIngress: false,
      k8sWorkloadKind: 'statefulSet',
      k8sGatewayEnabled: true,
      k8sGatewayName: 'edge-gw',
      k8sGatewayHost: 'api.example.com',
      k8sGatewayProvision: true,
      k8sVpaEnabled: true,
      k8sKedaEnabled: true,
      k8sCertManagerEnabled: true,
      k8sPdbMinAvailable: '2',
    });

    expect(yaml).toContain('workloadKind: statefulSet');
    expect(yaml).toContain('serviceAccountName: api-sa');
    expect(yaml).toContain('nodepool: apps');
    expect(yaml).toContain('provisionGateway: true');
    expect(yaml).toContain('verticalPodAutoscaler:');
    expect(yaml).toContain('keda:');
    expect(yaml).toContain('certManager:');
    expect(yaml).toContain('minAvailable: 2');
    expect(yaml).toContain('host: api.example.com');
  });
});
