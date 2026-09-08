// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

export type HelmFieldType = 'text' | 'password' | 'number' | 'boolean' | 'select';

export interface HelmFormField {
  key: string;
  label: string;
  type: HelmFieldType;
  defaultValue: string | boolean;
  placeholder?: string;
  options?: { label: string; value: string }[];
  help?: string;
}

export function helmFormFields(chartId: string): HelmFormField[] {
  switch (chartId) {
    case 'postgresql':
      return [
        { key: 'username', label: 'Database username', type: 'text', defaultValue: 'appuser' },
        { key: 'password', label: 'Database password', type: 'password', defaultValue: '' },
        { key: 'database', label: 'Database name', type: 'text', defaultValue: 'appdb' },
        { key: 'storageSize', label: 'Storage size', type: 'text', defaultValue: '20Gi' },
        { key: 'replication', label: 'High availability (replication)', type: 'boolean', defaultValue: false },
        { key: 'backup', label: 'Enable backup hooks', type: 'boolean', defaultValue: true },
      ];
    case 'redis':
      return [
        { key: 'password', label: 'Redis password', type: 'password', defaultValue: '' },
        { key: 'storageSize', label: 'Storage size', type: 'text', defaultValue: '8Gi' },
        { key: 'replicas', label: 'Replica count', type: 'number', defaultValue: '1' },
        { key: 'ha', label: 'Redis Sentinel HA', type: 'boolean', defaultValue: false },
      ];
    case 'prometheus':
      return [
        { key: 'retention', label: 'Retention', type: 'text', defaultValue: '15d' },
        { key: 'storageSize', label: 'Storage size', type: 'text', defaultValue: '50Gi' },
        { key: 'scrapeInterval', label: 'Scrape interval', type: 'text', defaultValue: '30s' },
      ];
    case 'grafana':
      return [
        { key: 'adminUser', label: 'Admin username', type: 'text', defaultValue: 'admin' },
        { key: 'adminPassword', label: 'Admin password', type: 'password', defaultValue: '' },
        { key: 'persistence', label: 'Enable persistence', type: 'boolean', defaultValue: true },
        { key: 'storageSize', label: 'Storage size', type: 'text', defaultValue: '10Gi' },
      ];
    case 'keycloak':
      return [
        { key: 'adminUser', label: 'Admin username', type: 'text', defaultValue: 'admin' },
        { key: 'adminPassword', label: 'Admin password', type: 'password', defaultValue: '' },
        { key: 'replicas', label: 'Replicas', type: 'number', defaultValue: '1' },
        { key: 'storageSize', label: 'Storage size', type: 'text', defaultValue: '10Gi' },
      ];
    case 'argo-cd':
      return [
        { key: 'insecure', label: 'Allow insecure server (dev)', type: 'boolean', defaultValue: false },
        { key: 'replicas', label: 'Server replicas', type: 'number', defaultValue: '1' },
      ];
    case 'nginx-ingress':
      return [
        { key: 'replicas', label: 'Controller replicas', type: 'number', defaultValue: '2' },
        { key: 'publishService', label: 'Publish service externally', type: 'boolean', defaultValue: true },
      ];
    case 'vault':
      return [
        { key: 'ha', label: 'High availability mode', type: 'boolean', defaultValue: false },
        { key: 'storageSize', label: 'Storage size', type: 'text', defaultValue: '10Gi' },
        { key: 'uiEnabled', label: 'Enable Vault UI', type: 'boolean', defaultValue: true },
      ];
    default:
      return [
        { key: 'storageSize', label: 'Storage size', type: 'text', defaultValue: '10Gi' },
      ];
  }
}

export function defaultFormValues(chartId: string): Record<string, string | boolean> {
  const fields = helmFormFields(chartId);
  return Object.fromEntries(fields.map((f) => [f.key, f.defaultValue]));
}

function yamlQuote(v: string): string {
  if (/[:#{}[\],&*!|>'"%@`]/.test(v) || v.includes('\n')) {
    return JSON.stringify(v);
  }
  return v;
}

/** Build Helm values YAML from wizard form fields (chart-specific mapping). */
export function buildValuesYaml(chartId: string, values: Record<string, string | boolean>): string {
  const s = (k: string) => String(values[k] ?? '');
  const b = (k: string) => values[k] === true || values[k] === 'true';

  switch (chartId) {
    case 'postgresql':
      return [
        'auth:',
        `  username: ${yamlQuote(s('username'))}`,
        s('password') ? `  password: ${yamlQuote(s('password'))}` : '  # password: set via secret',
        `  database: ${yamlQuote(s('database'))}`,
        'primary:',
        '  persistence:',
        '    enabled: true',
        `    size: ${yamlQuote(s('storageSize'))}`,
        b('replication') ? 'readReplicas:\n  replicaCount: 1' : '',
        b('backup') ? 'backup:\n  enabled: true' : '',
      ]
        .filter(Boolean)
        .join('\n');
    case 'redis':
      return [
        'auth:',
        s('password') ? `  password: ${yamlQuote(s('password'))}` : '  enabled: false',
        'master:',
        '  persistence:',
        '    enabled: true',
        `    size: ${yamlQuote(s('storageSize'))}`,
        `replica:\n  replicaCount: ${s('replicas') || '1'}`,
        b('ha') ? 'sentinel:\n  enabled: true' : '',
      ]
        .filter(Boolean)
        .join('\n');
    case 'prometheus':
      return [
        'server:',
        '  persistentVolume:',
        '    enabled: true',
        `    size: ${yamlQuote(s('storageSize'))}`,
        `  retention: ${yamlQuote(s('retention'))}`,
        'serverFiles:',
        '  prometheus.yml:',
        '    global:',
        `      scrape_interval: ${yamlQuote(s('scrapeInterval'))}`,
      ].join('\n');
    case 'grafana':
      return [
        'adminUser: ' + yamlQuote(s('adminUser')),
        s('adminPassword') ? `adminPassword: ${yamlQuote(s('adminPassword'))}` : '',
        'persistence:',
        `  enabled: ${b('persistence')}`,
        b('persistence') ? `  size: ${yamlQuote(s('storageSize'))}` : '',
      ]
        .filter(Boolean)
        .join('\n');
    case 'keycloak':
      return [
        'auth:',
        '  adminUser: ' + yamlQuote(s('adminUser')),
        s('adminPassword') ? `  adminPassword: ${yamlQuote(s('adminPassword'))}` : '',
        `replicaCount: ${s('replicas') || '1'}`,
        'postgresql:',
        '  enabled: true',
        '  primary:',
        '    persistence:',
        '      enabled: true',
        `      size: ${yamlQuote(s('storageSize'))}`,
      ]
        .filter(Boolean)
        .join('\n');
    case 'argo-cd':
      return [
        'server:',
        `  replicas: ${s('replicas') || '1'}`,
        `  insecure: ${b('insecure')}`,
      ].join('\n');
    case 'nginx-ingress':
      return [
        'controller:',
        `  replicaCount: ${s('replicas') || '2'}`,
        '  service:',
        `    enabled: ${b('publishService')}`,
      ].join('\n');
    case 'vault':
      return [
        'server:',
        '  standalone:',
        `    enabled: ${!b('ha')}`,
        '  ha:',
        `    enabled: ${b('ha')}`,
        '    raft:',
        '      enabled: true',
        '      config: |',
        '        ui = true',
        '  dataStorage:',
        '    enabled: true',
        `    size: ${yamlQuote(s('storageSize'))}`,
        `ui:\n  enabled: ${b('uiEnabled')}`,
      ].join('\n');
    default:
      return [
        'persistence:',
        '  enabled: true',
        `  size: ${yamlQuote(s('storageSize'))}`,
      ].join('\n');
  }
}
