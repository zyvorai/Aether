// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import {
  distinctPodImages,
  hasImageDrift,
  manifestAnnotations,
  manifestContainers,
  manifestLabels,
  manifestServicePorts,
} from './clusterManifest';

const deployment = {
  metadata: {
    labels: { app: 'web', tier: 'frontend' },
    annotations: {
      'deployed-by': 'aether',
      'kubectl.kubernetes.io/last-applied-configuration': '{"huge":true}',
    },
  },
  spec: {
    template: {
      spec: {
        containers: [
          {
            name: 'web',
            image: 'nginx:1.25',
            resources: {
              requests: { cpu: '100m', memory: '128Mi' },
              limits: { memory: '256Mi' },
            },
          },
        ],
      },
    },
  },
};

describe('clusterManifest helpers', () => {
  it('reads labels and filters last-applied annotations', () => {
    expect(manifestLabels(deployment)).toEqual({ app: 'web', tier: 'frontend' });
    expect(manifestAnnotations(deployment)).toEqual({ 'deployed-by': 'aether' });
    expect(manifestAnnotations(deployment, { includeLastApplied: true })['kubectl.kubernetes.io/last-applied-configuration']).toBeTruthy();
  });

  it('extracts container resources and service ports', () => {
    expect(manifestContainers(deployment)).toEqual([
      {
        name: 'web',
        image: 'nginx:1.25',
        requests: { cpu: '100m', memory: '128Mi' },
        limits: { cpu: undefined, memory: '256Mi' },
      },
    ]);
    expect(
      manifestServicePorts({
        spec: { ports: [{ name: 'http', port: 80, targetPort: 8080, protocol: 'TCP' }] },
      }),
    ).toEqual([{ name: 'http', port: 80, targetPort: 8080, protocol: 'TCP' }]);
  });

  it('detects image drift across pods', () => {
    expect(hasImageDrift([{ images: ['nginx:1.25'] }, { images: ['nginx:1.25'] }])).toBe(false);
    expect(hasImageDrift([{ images: ['nginx:1.24'] }, { images: ['nginx:1.25'] }])).toBe(true);
    expect(distinctPodImages([{ images: ['b'] }, { images: ['a', 'b'] }])).toEqual(['a', 'b']);
  });
});
