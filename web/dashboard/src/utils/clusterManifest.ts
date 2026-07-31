// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

type Json = Record<string, unknown>;

function asRecord(value: unknown): Json | null {
  return value && typeof value === 'object' && !Array.isArray(value) ? (value as Json) : null;
}

function asStringMap(value: unknown): Record<string, string> {
  const record = asRecord(value);
  if (!record) return {};
  const out: Record<string, string> = {};
  for (const [key, entry] of Object.entries(record)) {
    if (typeof entry === 'string') out[key] = entry;
  }
  return out;
}

/** Metadata labels from a Kubernetes resource manifest. */
export function manifestLabels(manifest: unknown): Record<string, string> {
  const meta = asRecord(asRecord(manifest)?.metadata);
  return asStringMap(meta?.labels);
}

/** Metadata annotations (drops last-applied blob unless requested). */
export function manifestAnnotations(
  manifest: unknown,
  options?: { includeLastApplied?: boolean },
): Record<string, string> {
  const meta = asRecord(asRecord(manifest)?.metadata);
  const all = asStringMap(meta?.annotations);
  if (options?.includeLastApplied) return all;
  const { ['kubectl.kubernetes.io/last-applied-configuration']: _drop, ...rest } = all;
  return rest;
}

export interface ResourceQuantity {
  cpu?: string;
  memory?: string;
}

export interface ContainerResources {
  name: string;
  requests: ResourceQuantity;
  limits: ResourceQuantity;
  image?: string;
}

function quantityFrom(resources: unknown, field: 'requests' | 'limits'): ResourceQuantity {
  const block = asRecord(asRecord(resources)?.[field]);
  return {
    cpu: typeof block?.cpu === 'string' ? block.cpu : undefined,
    memory: typeof block?.memory === 'string' ? block.memory : undefined,
  };
}

function containersFromSpec(spec: Json | null): unknown {
  if (!spec) return null;
  const templateSpec = asRecord(asRecord(spec.template)?.spec);
  if (Array.isArray(templateSpec?.containers)) return templateSpec.containers;
  const jobTemplateSpec = asRecord(
    asRecord(asRecord(asRecord(spec.jobTemplate)?.spec)?.template)?.spec,
  );
  if (Array.isArray(jobTemplateSpec?.containers)) return jobTemplateSpec.containers;
  if (Array.isArray(spec.containers)) return spec.containers;
  return null;
}

/** Resolve pod template containers (Deployment/STS/DS/Job) or Pod containers. */
export function manifestContainers(manifest: unknown): ContainerResources[] {
  const containers = containersFromSpec(asRecord(asRecord(manifest)?.spec));
  if (!Array.isArray(containers)) return [];

  const result: ContainerResources[] = [];
  for (const entry of containers) {
    const container = asRecord(entry);
    if (!container || typeof container.name !== 'string') continue;
    result.push({
      name: container.name,
      image: typeof container.image === 'string' ? container.image : undefined,
      requests: quantityFrom(container.resources, 'requests'),
      limits: quantityFrom(container.resources, 'limits'),
    });
  }
  return result;
}

/** First container's declared containerPort(s) — a better port-forward default
 * than a generic guess for workloads with no Service to read a port from. */
export function manifestContainerPorts(manifest: unknown): number[] {
  const containers = containersFromSpec(asRecord(asRecord(manifest)?.spec));
  if (!Array.isArray(containers)) return [];
  const result: number[] = [];
  for (const entry of containers) {
    const ports = asRecord(entry)?.ports;
    if (!Array.isArray(ports)) continue;
    for (const portEntry of ports) {
      const containerPort = asRecord(portEntry)?.containerPort;
      if (typeof containerPort === 'number') result.push(containerPort);
    }
  }
  return result;
}

export interface ServicePort {
  name?: string;
  port: number;
  targetPort?: string | number;
  protocol?: string;
}

/** Service ports from a Service manifest (for port-forward defaults). */
export function manifestServicePorts(manifest: unknown): ServicePort[] {
  const ports = asRecord(asRecord(manifest)?.spec)?.ports;
  if (!Array.isArray(ports)) return [];
  const result: ServicePort[] = [];
  for (const entry of ports) {
    const port = asRecord(entry);
    if (!port || typeof port.port !== 'number') continue;
    result.push({
      name: typeof port.name === 'string' ? port.name : undefined,
      port: port.port,
      targetPort:
        typeof port.targetPort === 'string' || typeof port.targetPort === 'number'
          ? port.targetPort
          : undefined,
      protocol: typeof port.protocol === 'string' ? port.protocol : undefined,
    });
  }
  return result;
}

/** Distinct container images observed on related pods. */
export function distinctPodImages(pods: Array<{ images?: string[] }>): string[] {
  const images = new Set<string>();
  for (const pod of pods) {
    for (const image of pod.images ?? []) {
      if (image) images.add(image);
    }
  }
  return Array.from(images).sort();
}

/** True when related pods are running more than one distinct image (rolling update / drift). */
export function hasImageDrift(pods: Array<{ images?: string[] }>): boolean {
  return distinctPodImages(pods).length > 1;
}

export function entriesSorted(map: Record<string, string>): Array<[string, string]> {
  return Object.entries(map).sort(([a], [b]) => a.localeCompare(b));
}
