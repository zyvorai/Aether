// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Helm chart export
//!
//! Generates a complete Helm chart directory from an Aether workload spec.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::Path;

use crate::spec::{ServiceType, Workload};

/// Curated Helm charts for the App Store UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelmCatalogChart {
    pub id: String,
    pub name: String,
    pub category: String,
    pub chart: String,
    pub repo: String,
    pub version: String,
    pub description: String,
    pub storage_required: bool,
    pub backup_supported: bool,
    pub ha_available: bool,
    pub monitoring_available: bool,
}

/// Static Helm catalog for dashboard App Store.
pub fn helm_catalog() -> Vec<HelmCatalogChart> {
    vec![
        HelmCatalogChart {
            id: "postgresql".into(),
            name: "PostgreSQL".into(),
            category: "Databases".into(),
            chart: "bitnami/postgresql".into(),
            repo: "https://charts.bitnami.com/bitnami".into(),
            version: "16".into(),
            description: "Production-grade PostgreSQL with backups and replication options.".into(),
            storage_required: true,
            backup_supported: true,
            ha_available: true,
            monitoring_available: true,
        },
        HelmCatalogChart {
            id: "redis".into(),
            name: "Redis".into(),
            category: "Databases".into(),
            chart: "bitnami/redis".into(),
            repo: "https://charts.bitnami.com/bitnami".into(),
            version: "7".into(),
            description: "In-memory cache and message broker.".into(),
            storage_required: true,
            backup_supported: true,
            ha_available: true,
            monitoring_available: true,
        },
        HelmCatalogChart {
            id: "prometheus".into(),
            name: "Prometheus".into(),
            category: "Observability".into(),
            chart: "prometheus-community/prometheus".into(),
            repo: "https://prometheus-community.github.io/helm-charts".into(),
            version: "25".into(),
            description: "Metrics collection and alerting stack.".into(),
            storage_required: true,
            backup_supported: false,
            ha_available: true,
            monitoring_available: true,
        },
        HelmCatalogChart {
            id: "grafana".into(),
            name: "Grafana".into(),
            category: "Observability".into(),
            chart: "grafana/grafana".into(),
            repo: "https://grafana.github.io/helm-charts".into(),
            version: "8".into(),
            description: "Dashboards and visualization for Prometheus/Loki.".into(),
            storage_required: false,
            backup_supported: true,
            ha_available: true,
            monitoring_available: true,
        },
        HelmCatalogChart {
            id: "keycloak".into(),
            name: "Keycloak".into(),
            category: "Security".into(),
            chart: "bitnami/keycloak".into(),
            repo: "https://charts.bitnami.com/bitnami".into(),
            version: "24".into(),
            description: "Identity and access management (OIDC/SAML).".into(),
            storage_required: true,
            backup_supported: true,
            ha_available: true,
            monitoring_available: true,
        },
        HelmCatalogChart {
            id: "argo-cd".into(),
            name: "Argo CD".into(),
            category: "DevOps".into(),
            chart: "argo/argo-cd".into(),
            repo: "https://argoproj.github.io/argo-helm".into(),
            version: "7".into(),
            description: "GitOps continuous delivery for Kubernetes.".into(),
            storage_required: false,
            backup_supported: true,
            ha_available: true,
            monitoring_available: true,
        },
        HelmCatalogChart {
            id: "nginx-ingress".into(),
            name: "NGINX Ingress".into(),
            category: "Networking".into(),
            chart: "ingress-nginx/ingress-nginx".into(),
            repo: "https://kubernetes.github.io/ingress-nginx".into(),
            version: "4".into(),
            description: "Ingress controller for HTTP/S routing.".into(),
            storage_required: false,
            backup_supported: false,
            ha_available: true,
            monitoring_available: true,
        },
        HelmCatalogChart {
            id: "vault".into(),
            name: "Vault".into(),
            category: "Security".into(),
            chart: "hashicorp/vault".into(),
            repo: "https://helm.releases.hashicorp.com".into(),
            version: "0.28".into(),
            description: "Secrets management and encryption.".into(),
            storage_required: true,
            backup_supported: true,
            ha_available: true,
            monitoring_available: true,
        },
    ]
}

/// Export a workload specification as a Helm chart.
///
/// Creates a complete Helm chart directory structure at `output_dir` containing
/// Chart.yaml, values.yaml, and templates for Deployment/CronJob, Service,
/// Ingress, HPA, and standard helpers.
pub fn export_helm_chart(
    spec: &Workload,
    output_dir: &Path,
    chart_version: Option<&str>,
) -> Result<()> {
    let name = &spec.metadata.name;
    let version = chart_version.unwrap_or("0.1.0");

    // Create directory structure
    let templates_dir = output_dir.join("templates");
    fs::create_dir_all(&templates_dir)
        .with_context(|| format!("Failed to create templates directory: {}", templates_dir.display()))?;

    // --- Chart.yaml ---
    write_file(
        &output_dir.join("Chart.yaml"),
        &generate_chart_yaml(name, version),
    )?;

    // --- values.yaml ---
    write_file(
        &output_dir.join("values.yaml"),
        &generate_values_yaml(spec),
    )?;

    // --- templates/_helpers.tpl ---
    write_file(
        &templates_dir.join("_helpers.tpl"),
        &generate_helpers_tpl(),
    )?;

    // --- templates/deployment.yaml or templates/cronjob.yaml ---
    if spec.schedule.is_some() {
        write_file(
            &templates_dir.join("cronjob.yaml"),
            &generate_cronjob_yaml(),
        )?;
    } else {
        write_file(
            &templates_dir.join("deployment.yaml"),
            &generate_deployment_yaml(),
        )?;
    }

    // --- templates/service.yaml ---
    write_file(
        &templates_dir.join("service.yaml"),
        &generate_service_yaml(),
    )?;

    // --- templates/ingress.yaml ---
    write_file(
        &templates_dir.join("ingress.yaml"),
        &generate_ingress_yaml(),
    )?;

    // --- templates/hpa.yaml (only if scaling is enabled) ---
    if let Some(ref scaling) = spec.scaling {
        if scaling.enabled {
            write_file(
                &templates_dir.join("hpa.yaml"),
                &generate_hpa_yaml(),
            )?;
        }
    }

    Ok(())
}

/// Write content to a file, creating it if necessary.
fn write_file(path: &Path, content: &str) -> Result<()> {
    let mut file = fs::File::create(path)
        .with_context(|| format!("Failed to create file: {}", path.display()))?;
    file.write_all(content.as_bytes())
        .with_context(|| format!("Failed to write to file: {}", path.display()))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Template generators
// ---------------------------------------------------------------------------

fn generate_chart_yaml(name: &str, version: &str) -> String {
    format!(
        r#"apiVersion: v2
name: {name}
description: Helm chart for {name} (generated by Aether)
type: application
version: {version}
appVersion: "1.0.0"
"#,
    )
}

fn generate_values_yaml(spec: &Workload) -> String {
    let image_full = spec.image_name(); // e.g. "ghcr.io/org/my-app:latest"
    // Split repo from tag, but only on the last ':' after the last '/'.
    // This avoids breaking on registry ports (e.g. "registry.io:5000/org/app:v1").
    let (repo, tag) = if let Some(slash_pos) = image_full.rfind('/') {
        let after_slash = &image_full[slash_pos..];
        if let Some(colon_offset) = after_slash.rfind(':') {
            let split_pos = slash_pos + colon_offset;
            (image_full[..split_pos].to_string(), image_full[split_pos + 1..].to_string())
        } else {
            (image_full.clone(), "latest".to_string())
        }
    } else {
        // No slash — simple "image:tag" or just "image"
        match image_full.rsplit_once(':') {
            Some((r, t)) => (r.to_string(), t.to_string()),
            None => (image_full.clone(), "latest".to_string()),
        }
    };

    let cpu_limit = &spec.requirements.cpu;
    let mem_limit = &spec.requirements.memory;
    let cpu_request = spec
        .requirements
        .cpu_request
        .as_deref()
        .unwrap_or(cpu_limit);
    let mem_request = spec
        .requirements
        .memory_request
        .as_deref()
        .unwrap_or(mem_limit);

    let replica_count = spec
        .scaling
        .as_ref()
        .map(|s| s.min_replicas)
        .unwrap_or(1);

    let mut out = String::new();

    // image
    out.push_str(&format!(
        r#"image:
  repository: {repo}
  tag: "{tag}"
  pullPolicy: IfNotPresent

replicaCount: {replica_count}

resources:
  limits:
    cpu: "{cpu_limit}"
    memory: "{mem_limit}"
  requests:
    cpu: "{cpu_request}"
    memory: "{mem_request}"
"#,
    ));

    // service
    let service_enabled = spec.network.service;
    let svc_type = match spec.network.service_type {
        ServiceType::ClusterIP | ServiceType::Headless => "ClusterIP",
        ServiceType::NodePort => "NodePort",
        ServiceType::LoadBalancer => "LoadBalancer",
        ServiceType::ExternalName => "ExternalName",
    };
    let svc_port = spec
        .network
        .ports
        .first()
        .map(|p| p.service_port)
        .unwrap_or(80);
    let container_port = spec
        .network
        .ports
        .first()
        .map(|p| p.container_port)
        .unwrap_or(svc_port);

    out.push_str(&format!(
        r#"
containerPort: {container_port}

service:
  enabled: {service_enabled}
  type: {svc_type}
  port: {svc_port}
"#,
    ));

    // ingress
    if let Some(ref ingress) = spec.ingress {
        out.push_str(&format!(
            r#"
ingress:
  enabled: {}
  host: "{}"
  tls: {}
"#,
            ingress.enabled, ingress.host, ingress.tls,
        ));
    } else {
        out.push_str(
            r#"
ingress:
  enabled: false
  host: ""
  tls: false
"#,
        );
    }

    // persistence
    if spec.persistence.enabled {
        out.push_str(&format!(
            r#"
persistence:
  enabled: true
  size: "{}"
"#,
            spec.persistence.size,
        ));
    } else {
        out.push_str(
            r#"
persistence:
  enabled: false
  size: "1Gi"
"#,
        );
    }

    // schedule
    if let Some(ref schedule) = spec.schedule {
        let restart = match schedule.restart_policy {
            crate::spec::JobRestartPolicy::Never => "Never",
            crate::spec::JobRestartPolicy::OnFailure => "OnFailure",
        };
        out.push_str(&format!(
            r#"
schedule:
  cron: "{}"
  restartPolicy: {restart}
"#,
            schedule.cron,
        ));
    }

    // scaling
    if let Some(ref scaling) = spec.scaling {
        if scaling.enabled {
            out.push_str(&format!(
                r#"
autoscaling:
  enabled: true
  minReplicas: {}
  maxReplicas: {}
"#,
                scaling.min_replicas, scaling.max_replicas,
            ));
        }
    }

    out
}

fn generate_helpers_tpl() -> String {
    r##"{{/*
Expand the name of the chart.
*/}}
{{- define "chart.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
*/}}
{{- define "chart.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "chart.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "chart.labels" -}}
helm.sh/chart: {{ include "chart.chart" . }}
{{ include "chart.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "chart.selectorLabels" -}}
app.kubernetes.io/name: {{ include "chart.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}
"##
    .to_string()
}

fn generate_deployment_yaml() -> String {
    r#"apiVersion: apps/v1
kind: Deployment
metadata:
  name: {{ include "chart.fullname" . }}
  labels:
    {{- include "chart.labels" . | nindent 4 }}
spec:
  replicas: {{ .Values.replicaCount }}
  selector:
    matchLabels:
      {{- include "chart.selectorLabels" . | nindent 6 }}
  template:
    metadata:
      labels:
        {{- include "chart.selectorLabels" . | nindent 8 }}
    spec:
      containers:
        - name: {{ include "chart.name" . }}
          image: "{{ .Values.image.repository }}:{{ .Values.image.tag }}"
          imagePullPolicy: {{ .Values.image.pullPolicy }}
          ports:
            - containerPort: {{ .Values.containerPort }}
              protocol: TCP
          resources:
            limits:
              cpu: {{ .Values.resources.limits.cpu }}
              memory: {{ .Values.resources.limits.memory }}
            requests:
              cpu: {{ .Values.resources.requests.cpu }}
              memory: {{ .Values.resources.requests.memory }}
          {{- with .Values.env }}
          env:
            {{- toYaml . | nindent 12 }}
          {{- end }}
      {{- if .Values.persistence.enabled }}
          volumeMounts:
            - name: data
              mountPath: /data
      volumes:
        - name: data
          persistentVolumeClaim:
            claimName: {{ include "chart.fullname" . }}-pvc
      {{- end }}
"#
    .to_string()
}

fn generate_cronjob_yaml() -> String {
    r#"apiVersion: batch/v1
kind: CronJob
metadata:
  name: {{ include "chart.fullname" . }}
  labels:
    {{- include "chart.labels" . | nindent 4 }}
spec:
  schedule: {{ .Values.schedule.cron | quote }}
  jobTemplate:
    spec:
      template:
        metadata:
          labels:
            {{- include "chart.selectorLabels" . | nindent 12 }}
        spec:
          restartPolicy: {{ .Values.schedule.restartPolicy | default "Never" }}
          containers:
            - name: {{ include "chart.name" . }}
              image: "{{ .Values.image.repository }}:{{ .Values.image.tag }}"
              imagePullPolicy: {{ .Values.image.pullPolicy }}
              resources:
                limits:
                  cpu: {{ .Values.resources.limits.cpu }}
                  memory: {{ .Values.resources.limits.memory }}
                requests:
                  cpu: {{ .Values.resources.requests.cpu }}
                  memory: {{ .Values.resources.requests.memory }}
"#
    .to_string()
}

fn generate_service_yaml() -> String {
    r#"{{- if .Values.service.enabled }}
apiVersion: v1
kind: Service
metadata:
  name: {{ include "chart.fullname" . }}
  labels:
    {{- include "chart.labels" . | nindent 4 }}
spec:
  type: {{ .Values.service.type }}
  ports:
    - port: {{ .Values.service.port }}
      targetPort: {{ .Values.containerPort }}
      protocol: TCP
      name: http
  selector:
    {{- include "chart.selectorLabels" . | nindent 4 }}
{{- end }}
"#
    .to_string()
}

fn generate_ingress_yaml() -> String {
    r#"{{- if .Values.ingress.enabled }}
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: {{ include "chart.fullname" . }}
  labels:
    {{- include "chart.labels" . | nindent 4 }}
spec:
  {{- if .Values.ingress.tls }}
  tls:
    - hosts:
        - {{ .Values.ingress.host | quote }}
      secretName: {{ include "chart.fullname" . }}-tls
  {{- end }}
  rules:
    - host: {{ .Values.ingress.host | quote }}
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: {{ include "chart.fullname" . }}
                port:
                  number: {{ .Values.service.port }}
{{- end }}
"#
    .to_string()
}

fn generate_hpa_yaml() -> String {
    r#"{{- if .Values.autoscaling.enabled }}
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: {{ include "chart.fullname" . }}
  labels:
    {{- include "chart.labels" . | nindent 4 }}
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: {{ include "chart.fullname" . }}
  minReplicas: {{ .Values.autoscaling.minReplicas }}
  maxReplicas: {{ .Values.autoscaling.maxReplicas }}
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: {{ .Values.autoscaling.targetCPUUtilization | default 80 }}
{{- end }}
"#
    .to_string()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    /// Create a minimal valid workload for testing.
    fn make_workload() -> Workload {
        Workload {
            api_version: "aether/v1".to_string(),
            kind: "Workload".to_string(),
            metadata: Metadata {
                name: "test-app".to_string(),
                owner: "team".to_string(),
                project: "demo".to_string(),
                labels: HashMap::new(),
                annotations: HashMap::new(),
            },
            build: BuildSpec {
                context: PathBuf::from("."),
                dockerfile: PathBuf::from("Dockerfile"),
                registry: "ghcr.io/myorg".to_string(),
                build_args: HashMap::new(),
            ..Default::default()
            },
            requirements: ResourceRequirements {
                cpu: "2".to_string(),
                memory: "4Gi".to_string(),
                storage: "20Gi".to_string(),
                gpu: None,
                cpu_request: Some("500m".to_string()),
                memory_request: Some("1Gi".to_string()),
            },
            runtime: RuntimeSpec {
                preferred: RuntimePreference::Auto,
                allow: vec![RuntimeType::Container, RuntimeType::Kube],
            },
            network: NetworkSpec {
                service: true,
                service_type: ServiceType::ClusterIP,
                ports: vec![PortMapping {
                    container_port: 8080,
                    service_port: 80,
                    protocol: "TCP".to_string(),
                }],
                network_policy: None,
        ..Default::default()
            },
            persistence: PersistenceSpec::default(),
            health: None,
            config: None,
            ingress: None,
            scaling: None,
            mesh: None,
            intent: None,
            autonomy: None,
            confidential: None,
            schedule: None,
        kubernetes: None,
        }
    }

    #[test]
    fn test_basic_chart_generation() {
        let dir = tempfile::tempdir().unwrap();
        let spec = make_workload();

        export_helm_chart(&spec, dir.path(), None).unwrap();

        // Chart.yaml
        let chart = fs::read_to_string(dir.path().join("Chart.yaml")).unwrap();
        assert!(chart.contains("apiVersion: v2"));
        assert!(chart.contains("name: test-app"));
        assert!(chart.contains("version: 0.1.0"));
        assert!(chart.contains("appVersion: \"1.0.0\""));
        assert!(chart.contains("type: application"));

        // values.yaml
        let values = fs::read_to_string(dir.path().join("values.yaml")).unwrap();
        assert!(values.contains("repository: ghcr.io/myorg/test-app"));
        assert!(values.contains("tag: \"latest\""));
        assert!(values.contains("replicaCount: 1"));
        assert!(values.contains("cpu: \"2\""));
        assert!(values.contains("memory: \"4Gi\""));
        assert!(values.contains("cpu: \"500m\""));
        assert!(values.contains("memory: \"1Gi\""));
        assert!(values.contains("service:"));
        assert!(values.contains("enabled: true"));
        assert!(values.contains("type: ClusterIP"));
        assert!(values.contains("port: 80"));

        // templates/deployment.yaml exists
        let deploy = fs::read_to_string(dir.path().join("templates/deployment.yaml")).unwrap();
        assert!(deploy.contains("kind: Deployment"));
        assert!(deploy.contains("{{ .Values.image.repository }}"));
        assert!(deploy.contains("{{ .Values.replicaCount }}"));

        // templates/service.yaml exists
        let svc = fs::read_to_string(dir.path().join("templates/service.yaml")).unwrap();
        assert!(svc.contains("kind: Service"));
        assert!(svc.contains("{{- if .Values.service.enabled }}"));

        // templates/ingress.yaml exists
        let ing = fs::read_to_string(dir.path().join("templates/ingress.yaml")).unwrap();
        assert!(ing.contains("{{- if .Values.ingress.enabled }}"));

        // templates/_helpers.tpl exists
        let helpers = fs::read_to_string(dir.path().join("templates/_helpers.tpl")).unwrap();
        assert!(helpers.contains("chart.fullname"));
        assert!(helpers.contains("chart.labels"));

        // No cronjob or hpa for this basic workload
        assert!(!dir.path().join("templates/cronjob.yaml").exists());
        assert!(!dir.path().join("templates/hpa.yaml").exists());
    }

    #[test]
    fn test_chart_with_custom_version() {
        let dir = tempfile::tempdir().unwrap();
        let spec = make_workload();

        export_helm_chart(&spec, dir.path(), Some("2.3.4")).unwrap();

        let chart = fs::read_to_string(dir.path().join("Chart.yaml")).unwrap();
        assert!(chart.contains("version: 2.3.4"));
    }

    #[test]
    fn test_chart_with_ingress() {
        let dir = tempfile::tempdir().unwrap();
        let mut spec = make_workload();
        spec.ingress = Some(IngressSpec {
            enabled: true,
            host: "app.example.com".to_string(),
            paths: vec![],
            tls: true,
            annotations: HashMap::new(),
            ..Default::default()
        });

        export_helm_chart(&spec, dir.path(), None).unwrap();

        let values = fs::read_to_string(dir.path().join("values.yaml")).unwrap();
        assert!(values.contains("ingress:"));
        assert!(values.contains("enabled: true"));
        assert!(values.contains("host: \"app.example.com\""));
        assert!(values.contains("tls: true"));

        let ing = fs::read_to_string(dir.path().join("templates/ingress.yaml")).unwrap();
        assert!(ing.contains("kind: Ingress"));
        assert!(ing.contains("networking.k8s.io/v1"));
        assert!(ing.contains("{{ .Values.ingress.host"));
    }

    #[test]
    fn test_chart_with_schedule_generates_cronjob() {
        let dir = tempfile::tempdir().unwrap();
        let mut spec = make_workload();
        spec.schedule = Some(ScheduleSpec {
            cron: "*/15 * * * *".to_string(),
            concurrency_policy: ConcurrencyPolicy::Allow,
            backoff_limit: 3,
            active_deadline_seconds: None,
            restart_policy: JobRestartPolicy::Never,
        });

        export_helm_chart(&spec, dir.path(), None).unwrap();

        // Should generate cronjob, not deployment
        assert!(dir.path().join("templates/cronjob.yaml").exists());
        assert!(!dir.path().join("templates/deployment.yaml").exists());

        let cron = fs::read_to_string(dir.path().join("templates/cronjob.yaml")).unwrap();
        assert!(cron.contains("kind: CronJob"));
        assert!(cron.contains("{{ .Values.schedule.cron"));

        let values = fs::read_to_string(dir.path().join("values.yaml")).unwrap();
        assert!(values.contains("schedule:"));
        assert!(values.contains("cron: \"*/15 * * * *\""));
    }

    #[test]
    fn test_chart_with_scaling_generates_hpa() {
        let dir = tempfile::tempdir().unwrap();
        let mut spec = make_workload();
        spec.scaling = Some(ScalingSpec {
            enabled: true,
            min_replicas: 2,
            max_replicas: 10,
            metrics: vec![],
            ..Default::default()
        });

        export_helm_chart(&spec, dir.path(), None).unwrap();

        assert!(dir.path().join("templates/hpa.yaml").exists());

        let hpa = fs::read_to_string(dir.path().join("templates/hpa.yaml")).unwrap();
        assert!(hpa.contains("kind: HorizontalPodAutoscaler"));
        assert!(hpa.contains("{{ .Values.autoscaling.minReplicas }}"));
        assert!(hpa.contains("{{ .Values.autoscaling.maxReplicas }}"));

        let values = fs::read_to_string(dir.path().join("values.yaml")).unwrap();
        assert!(values.contains("autoscaling:"));
        assert!(values.contains("enabled: true"));
        assert!(values.contains("minReplicas: 2"));
        assert!(values.contains("maxReplicas: 10"));
        // replicaCount should use min_replicas
        assert!(values.contains("replicaCount: 2"));
    }

    #[test]
    fn test_chart_scaling_disabled_no_hpa() {
        let dir = tempfile::tempdir().unwrap();
        let mut spec = make_workload();
        spec.scaling = Some(ScalingSpec {
            enabled: false,
            min_replicas: 1,
            max_replicas: 5,
            metrics: vec![],
            ..Default::default()
        });

        export_helm_chart(&spec, dir.path(), None).unwrap();

        assert!(!dir.path().join("templates/hpa.yaml").exists());
    }

    #[test]
    fn test_chart_with_persistence() {
        let dir = tempfile::tempdir().unwrap();
        let mut spec = make_workload();
        spec.persistence = PersistenceSpec {
            enabled: true,
            size: "50Gi".to_string(),
            access_mode: AccessMode::ReadWriteOnce,
            storage_class: None,
        };

        export_helm_chart(&spec, dir.path(), None).unwrap();

        let values = fs::read_to_string(dir.path().join("values.yaml")).unwrap();
        assert!(values.contains("persistence:"));
        assert!(values.contains("enabled: true"));
        assert!(values.contains("size: \"50Gi\""));
    }

    #[test]
    fn test_chart_loadbalancer_service() {
        let dir = tempfile::tempdir().unwrap();
        let mut spec = make_workload();
        spec.network.service_type = ServiceType::LoadBalancer;

        export_helm_chart(&spec, dir.path(), None).unwrap();

        let values = fs::read_to_string(dir.path().join("values.yaml")).unwrap();
        assert!(values.contains("type: LoadBalancer"));
    }

    #[test]
    fn test_chart_no_ingress_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let spec = make_workload(); // no ingress

        export_helm_chart(&spec, dir.path(), None).unwrap();

        let values = fs::read_to_string(dir.path().join("values.yaml")).unwrap();
        assert!(values.contains("ingress:"));
        assert!(values.contains("enabled: false"));
    }
}
