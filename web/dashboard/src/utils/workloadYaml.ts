/** Build and parse aether/v1 workload YAML for deploy and validate modals. */

export interface EditorWorkloadInput {
  name: string;
  image: string;
  runtime: string;
  replicas: number;
  cpu: string;
  memory: string;
  intent: string;
  healthCheck: boolean;
  owner?: string;
  project?: string;
  confidentialEnabled?: boolean;
  confidentialTee?: 'sev-snp' | 'tdx';
  /** Tenant-facing profile; when set, backend resolves kataRuntimeClass. */
  confidentialSecurityProfile?: '' | 'sandbox' | 'standard-confidential' | 'sovereign-high';
  confidentialKataRuntime?: 'kata-clh-snp' | 'kata-clh-tdx' | 'kata-qemu-snp' | 'kata-qemu-tdx';
  attestationRequired?: boolean;
  imageDigest?: string;
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

function appendConfidentialBlock(
  lines: string[],
  input: EditorWorkloadInput,
  preferred: string,
  kataPath: boolean,
) {
  if (!input.confidentialEnabled) return;
  const tee = input.confidentialTee ?? 'sev-snp';
  lines.push(
    'confidential:',
    '  enabled: true',
    `  tee: ${tee}`,
    '  attestation:',
    `    required: ${input.attestationRequired !== false}`,
    '    policy: strict',
    '  isolation:',
    '    vtpm: true',
    '    encryptedState: true',
    '    debugAllowed: false',
    '  secrets:',
    '    releasePolicy: attest-gated',
    '    provider: vault',
  );
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

  if (input.replicas > 1) {
    lines.push(
      'scaling:',
      '  enabled: true',
      `  minReplicas: ${input.replicas}`,
      `  maxReplicas: ${input.replicas}`,
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
