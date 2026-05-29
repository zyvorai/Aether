// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

/** Build and parse aether/v1 workload YAML for deploy and validate modals. */

export interface EditorWorkloadInput {
  name: string;
  image: string;
  runtime: string;
  replicas: number;
  cpu: string;
  memory: string;
  intent: string;
  env?: string;
  healthCheck: boolean;
  owner?: string;
  project?: string;
  confidentialEnabled?: boolean;
  confidentialTee?: 'sev-snp' | 'tdx';
  /** Tenant-facing profile; when set, backend resolves kataRuntimeClass. */
  confidentialSecurityProfile?: string;
  confidentialKataRuntime?: 'kata-clh-snp' | 'kata-clh-tdx' | 'kata-qemu-snp' | 'kata-qemu-tdx';
  attestationRequired?: boolean;
  attestationPolicy?: 'strict' | 'standard';
  confidentialRegionLock?: string;
  confidentialSecretNames?: string;
  confidentialVtpm?: boolean;
  confidentialEncryptedState?: boolean;
  confidentialDebugAllowed?: boolean;
  imageDigest?: string;
  k8sNamespace?: string;
  k8sServiceAccount?: string;
  k8sNodeSelector?: string;
  scalingEnabled?: boolean;
  scalingMin?: number;
  scalingMax?: number;
  ingressEnabled?: boolean;
  ingressHost?: string;
  ingressPath?: string;
  networkDenyAllIngress?: boolean;
  k8sWorkloadKind?: string;
  k8sGatewayEnabled?: boolean;
  k8sGatewayName?: string;
  k8sGatewayHost?: string;
  k8sGatewayProvision?: boolean;
  k8sVpaEnabled?: boolean;
  k8sKedaEnabled?: boolean;
  k8sCertManagerEnabled?: boolean;
  k8sPdbMinAvailable?: string;
}

export const DEFAULT_DEPLOY_WORKLOAD_YAML = `apiVersion: aether/v1
kind: Workload
metadata:
  name: httpd
  owner: dashboard
  project: default
build:
  context: .
  dockerfile: Dockerfile
  registry: docker.io/library
  tag: latest
requirements:
  cpu: 500m
  memory: 512Mi
  storage: 1Gi
runtime:
  preferred: kube
  allow:
    - kube
health:
  liveness:
    httpGet:
      path: /
      port: 80
    initialDelaySeconds: 10
    periodSeconds: 10
  readiness:
    httpGet:
      path: /
      port: 80
    initialDelaySeconds: 5
    periodSeconds: 5
network:
  service: true
  ports:
    - containerPort: 80
      servicePort: 80
      protocol: TCP
`;

/** Prefill YAML for a new deploy (stable pullable image: docker.io/library/httpd:latest). */
export function freshDeployWorkloadYaml(): string {
  return DEFAULT_DEPLOY_WORKLOAD_YAML;
}

export function workloadNameFromYaml(yaml: string): string | null {
  const v1 = yaml.match(/metadata:\s*\n(?:[ \t][^\n]*\n)*?[ \t]+name:\s*(\S+)/);
  if (v1) return v1[1];
  const legacy = yaml.match(/^name:\s*(\S+)/m);
  return legacy?.[1] ?? null;
}

function runtimeBlock(runtime: string): { preferred: string; allow: string[] } {
  switch (runtime.toLowerCase()) {
    case 'podman':
    case 'docker':
      return { preferred: 'container', allow: ['container'] };
    case 'kubernetes':
    case 'kube':
      return { preferred: 'kube', allow: ['kube'] };
    case 'kata':
      return { preferred: 'kube', allow: ['kube'] };
    case 'kubevirt':
      return { preferred: 'kubevirt', allow: ['kubevirt'] };
    case 'metal3':
    case 'metal':
      return { preferred: 'metal', allow: ['metal'] };
    default:
      return { preferred: 'auto', allow: ['container', 'kube', 'kubevirt', 'metal'] };
  }
}

function intentGoal(intent: string): string {
  switch (intent.toLowerCase()) {
    case 'low-latency':
      return 'low-latency';
    case 'high-throughput':
      return 'high-throughput';
    case 'cost-optimized':
      return 'cost-optimized';
    default:
      return 'balanced';
  }
}

function parseImageRef(image: string): { registry: string; tag: string; repo: string } {
  const trimmed = image.trim();
  let tag = 'latest';
  let rest = trimmed;
  const colon = trimmed.lastIndexOf(':');
  if (colon > 0) {
    const base = trimmed.slice(0, colon);
    if (!base.includes('/') || base.includes('.') || base.startsWith('localhost')) {
      rest = base;
      tag = trimmed.slice(colon + 1);
    }
  }
  const parts = rest.split('/');
  if (parts.length === 1) {
    return { registry: 'docker.io/library', tag, repo: parts[0] };
  }
  if (parts.length === 2 && (parts[0].includes('.') || parts[0].includes(':') || parts[0] === 'localhost')) {
    return { registry: parts[0], tag, repo: parts[1] };
  }
  const repo = parts[parts.length - 1];
  return { registry: parts.slice(0, -1).join('/'), tag, repo };
}

function parseEnvLines(envText: string): Record<string, string> {
  const out: Record<string, string> = {};
  for (const line of envText.split('\n')) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;
    const eq = trimmed.indexOf('=');
    if (eq <= 0) continue;
    out[trimmed.slice(0, eq).trim()] = trimmed.slice(eq + 1).trim();
  }
  return out;
}

function appendEnvConfig(lines: string[], envText: string | undefined, mapName: string) {
  const envMap = parseEnvLines(envText ?? '');
  if (Object.keys(envMap).length === 0) return;
  lines.push('config:');
  lines.push('  configMaps:');
  lines.push(`    - name: ${mapName}`);
  lines.push('      data:');
  for (const [key, value] of Object.entries(envMap)) {
    lines.push(`        ${key}: ${value}`);
  }
  lines.push('  envFrom:');
  lines.push('    - sourceType: ConfigMap');
  lines.push(`      name: ${mapName}`);
}

function buildConfidentialYamlLines(
  input: EditorWorkloadInput,
  preferred: string,
  kataPath: boolean,
): string[] {
  if (!input.confidentialEnabled) return [];
  const tee = input.confidentialTee ?? 'sev-snp';
  const policy = input.attestationPolicy ?? 'strict';
  const vtpm = input.confidentialVtpm !== false;
  const encryptedState = input.confidentialEncryptedState !== false;
  const debugAllowed = input.confidentialDebugAllowed === true;
  const lines = [
    'confidential:',
    '  enabled: true',
    `  tee: ${tee}`,
    '  attestation:',
    `    required: ${input.attestationRequired !== false}`,
    `    policy: ${policy}`,
    '  isolation:',
    `    vtpm: ${vtpm}`,
    `    encryptedState: ${encryptedState}`,
    `    debugAllowed: ${debugAllowed}`,
    '  secrets:',
    '    releasePolicy: attest-gated',
    '    provider: vault',
  ];
  const secretNames = (input.confidentialSecretNames ?? '')
    .split(/[\n,]+/)
    .map((s) => s.trim())
    .filter(Boolean);
  if (secretNames.length > 0) {
    lines.push('    names:');
    for (const name of secretNames) {
      lines.push(`      - ${name}`);
    }
  }
  if (input.confidentialRegionLock?.trim()) {
    lines.push(`  regionLock: ${input.confidentialRegionLock.trim()}`);
  }
  if (input.imageDigest?.trim()) {
    lines.push(`  imageDigest: ${input.imageDigest.trim()}`);
  }
  const profile = input.confidentialSecurityProfile?.trim();
  if (profile) {
    lines.push(`  securityProfile: ${profile}`);
  } else if (kataPath && input.confidentialKataRuntime) {
    lines.push(`  kataRuntimeClass: ${input.confidentialKataRuntime}`);
  }
  if (preferred === 'kubevirt') {
    // kubevirt path — no extra kata field
  }
  return lines;
}

function appendConfidentialBlock(
  lines: string[],
  input: EditorWorkloadInput,
  preferred: string,
  kataPath: boolean,
) {
  lines.push(...buildConfidentialYamlLines(input, preferred, kataPath));
}

function inferRuntimeFromYaml(yaml: string): string | null {
  if (/preferred:\s*kubevirt/i.test(yaml) || /-\s*kubevirt/i.test(yaml)) return 'kubevirt';
  if (/preferred:\s*kata/i.test(yaml)) return 'kata';
  if (/preferred:\s*kube/i.test(yaml)) return 'kubernetes';
  return null;
}

/** Merge or replace the confidential block in existing workload YAML. */
export function mergeConfidentialIntoYaml(yaml: string, input: Partial<EditorWorkloadInput>): string {
  const runtime = inferRuntimeFromYaml(yaml) ?? 'kubernetes';
  const body = yaml.replace(/^confidential:\n(?:^  .+\n?)+/m, '').trimEnd();
  if (!input.confidentialEnabled) {
    return body;
  }
  const { preferred } = runtimeBlock(runtime);
  const kataPath = preferred === 'kube' && (runtime === 'kata' || runtime === 'kubernetes');
  const block = buildConfidentialYamlLines(input as EditorWorkloadInput, preferred, kataPath);
  if (block.length === 0) return body;
  return `${body}\n\n${block.join('\n')}\n`;
}

/** Generate aether/v1 workload YAML from the visual editor form. */
export function buildEditorWorkloadYaml(input: EditorWorkloadInput): string {
  const { preferred, allow } = runtimeBlock(input.runtime);
  const { registry, tag, repo } = parseImageRef(input.image);
  const metadataName = input.name.trim() || repo;
  const owner = input.owner?.trim() || 'dashboard';
  const project = input.project?.trim() || 'default';
  const kataPath = preferred === 'kube' && (input.runtime === 'kata' || input.runtime === 'kubernetes');

  const lines: string[] = [
    'apiVersion: aether/v1',
    'kind: Workload',
    'metadata:',
    `  name: ${metadataName}`,
    `  owner: ${owner}`,
    `  project: ${project}`,
  ];
  if (input.k8sNamespace?.trim() && preferred === 'kube') {
    lines.push(`  namespace: ${input.k8sNamespace.trim()}`);
  }
  if (input.image.trim() && metadataName !== input.name.trim()) {
    lines.push(`  labels:`);
    lines.push(`    aether.io/display-name: ${input.name.trim()}`);
  }
  lines.push(
    'build:',
    '  context: .',
    '  dockerfile: Dockerfile',
    `  registry: ${registry}`,
    `  tag: ${tag}`,
    'requirements:',
    `  cpu: ${input.cpu}`,
    `  memory: ${input.memory}`,
    '  storage: 1Gi',
    'runtime:',
    `  preferred: ${preferred}`,
    '  allow:',
    ...allow.map((r) => `    - ${r}`),
  );

  if (input.scalingEnabled || input.replicas > 1) {
    const minR = input.scalingEnabled ? (input.scalingMin ?? 1) : input.replicas;
    const maxR = input.scalingEnabled
      ? (input.scalingMax ?? Math.max(minR, input.replicas))
      : input.replicas;
    lines.push(
      'scaling:',
      '  enabled: true',
      `  minReplicas: ${minR}`,
      `  maxReplicas: ${maxR}`,
    );
  }

  const confidentialStrict = input.confidentialEnabled && (preferred === 'kubevirt' || kataPath);
  if (input.intent || confidentialStrict) {
    lines.push('intent:');
    if (input.intent) {
      lines.push(`  goal: ${intentGoal(input.intent)}`);
    }
    if (confidentialStrict) {
      lines.push('  trust: strict', '  compliance:', '    isolationRequired: true');
    }
  }

  appendConfidentialBlock(lines, input, preferred, kataPath);

  if (preferred === 'kube' && (input.k8sNamespace?.trim() || input.k8sServiceAccount?.trim() || input.k8sNodeSelector?.trim() || input.k8sWorkloadKind || input.k8sGatewayEnabled || input.k8sVpaEnabled || input.k8sKedaEnabled || input.k8sCertManagerEnabled || input.k8sPdbMinAvailable?.trim())) {
    lines.push('kubernetes:');
    if (input.k8sWorkloadKind && input.k8sWorkloadKind !== 'deployment') {
      lines.push(`  workloadKind: ${input.k8sWorkloadKind}`);
    }
    if (input.k8sServiceAccount?.trim()) {
      lines.push(`  serviceAccountName: ${input.k8sServiceAccount.trim()}`);
    }
    if (input.k8sNodeSelector?.trim()) {
      const parts = input.k8sNodeSelector.split('=');
      if (parts.length >= 2) {
        lines.push('  nodeSelector:');
        lines.push(`    ${parts[0].trim()}: ${parts.slice(1).join('=').trim()}`);
      }
    }
    if (input.k8sGatewayEnabled && input.k8sGatewayName?.trim() && input.k8sGatewayHost?.trim()) {
      lines.push('  gateway:');
      lines.push('    enabled: true');
      lines.push(`    gatewayName: ${input.k8sGatewayName.trim()}`);
      lines.push(`    host: ${input.k8sGatewayHost.trim()}`);
      if (input.k8sGatewayProvision) {
        lines.push('    provisionGateway: true');
      }
    }
    if (input.k8sVpaEnabled) {
      lines.push('  verticalPodAutoscaler:');
      lines.push('    enabled: true');
      lines.push('    updateMode: Auto');
    }
    if (input.k8sKedaEnabled) {
      lines.push('  keda:');
      lines.push('    enabled: true');
    }
    if (input.k8sCertManagerEnabled && input.ingressHost?.trim()) {
      lines.push('  certManager:');
      lines.push('    enabled: true');
      lines.push('    issuerName: letsencrypt-prod');
      lines.push('    issuerKind: ClusterIssuer');
    }
    if (input.k8sPdbMinAvailable?.trim()) {
      lines.push('  podDisruptionBudget:');
      lines.push(`    minAvailable: ${input.k8sPdbMinAvailable.trim()}`);
    }
  }

  appendEnvConfig(lines, input.env, `${metadataName}-env`);

  if (input.healthCheck) {
    lines.push(
      'health:',
      '  readiness:',
      '    httpGet:',
      '      path: /health',
      '      port: 80',
      '    initialDelaySeconds: 5',
      '    periodSeconds: 10',
    );
  }

  if (preferred === 'kube' || preferred === 'container') {
    lines.push(
      'network:',
      '  service: true',
      '  ports:',
      '    - containerPort: 80',
      '      servicePort: 80',
      '      protocol: TCP',
    );
    if (input.networkDenyAllIngress) {
      lines.push('  networkPolicy:');
      lines.push('    denyAllIngress: true');
    }
  }

  if (input.ingressEnabled && input.ingressHost?.trim()) {
    lines.push(
      'ingress:',
      '  enabled: true',
      `  host: ${input.ingressHost.trim()}`,
      '  paths:',
      `    - path: ${input.ingressPath?.trim() || '/'}`,
      '      pathType: Prefix',
      '      port: 80',
    );
  }

  return lines.join('\n');
}

function yamlScalar(value: unknown, indent: number): string[] {
  const pad = ' '.repeat(indent);
  if (value === null || value === undefined) return [];
  if (typeof value === 'boolean' || typeof value === 'number') {
    return [`${pad}${value}`];
  }
  if (typeof value === 'string') {
    return [`${pad}${value}`];
  }
  if (Array.isArray(value)) {
    const out: string[] = [];
    for (const item of value) {
      if (item !== null && typeof item === 'object' && !Array.isArray(item)) {
        out.push(`${pad}-`);
        out.push(...yamlObject(item as Record<string, unknown>, indent + 2));
      } else {
        out.push(`${pad}- ${item}`);
      }
    }
    return out;
  }
  if (typeof value === 'object') {
    return yamlObject(value as Record<string, unknown>, indent);
  }
  return [`${pad}${String(value)}`];
}

function yamlObject(obj: Record<string, unknown>, indent: number): string[] {
  const lines: string[] = [];
  for (const [key, value] of Object.entries(obj)) {
    if (value === undefined) continue;
    if (value !== null && typeof value === 'object' && !Array.isArray(value)) {
      lines.push(`${' '.repeat(indent)}${key}:`);
      lines.push(...yamlObject(value as Record<string, unknown>, indent + 2));
    } else if (Array.isArray(value)) {
      lines.push(`${' '.repeat(indent)}${key}:`);
      lines.push(...yamlScalar(value, indent + 2));
    } else {
      lines.push(`${' '.repeat(indent)}${key}: ${value}`);
    }
  }
  return lines;
}

/** Convert template API JSON (camelCase Workload) to YAML for the deploy modal. */
export function workloadJsonToYaml(spec: Record<string, unknown>): string {
  const lines = yamlObject(spec, 0);
  return lines.join('\n');
}
