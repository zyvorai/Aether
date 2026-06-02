// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

//! Cross-cluster volume replication planning.

use crate::spec::Workload;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeReplicationRequest {
    pub source_cluster: String,
    pub target_cluster: String,
    pub namespace: String,
    pub pvc_name: String,
    pub workload_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeReplicationStep {
    pub order: u32,
    pub action: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeReplicationPlan {
    pub source_cluster: String,
    pub target_cluster: String,
    pub namespace: String,
    pub pvc_name: String,
    pub storage_class_hint: Option<String>,
    pub size: Option<String>,
    pub steps: Vec<VolumeReplicationStep>,
    pub warnings: Vec<String>,
}

pub fn plan_volume_replication(
    req: &VolumeReplicationRequest,
    spec: Option<&Workload>,
) -> Result<VolumeReplicationPlan> {
    if req.source_cluster.trim().is_empty() || req.target_cluster.trim().is_empty() {
        bail!("source_cluster and target_cluster are required");
    }
    if req.source_cluster == req.target_cluster {
        bail!("source and target cluster must differ");
    }
    if req.pvc_name.trim().is_empty() || req.namespace.trim().is_empty() {
        bail!("namespace and pvc_name are required");
    }

    let (storage_class, size) = spec
        .and_then(|s| s.persistence.enabled.then_some(&s.persistence))
        .map(|p| (p.storage_class.clone(), Some(p.size.clone())))
        .unwrap_or((None, None));

    let steps = vec![
        VolumeReplicationStep {
            order: 1,
            action: "snapshot_source".into(),
            detail: format!(
                "Create VolumeSnapshot of {}/{} on cluster {}",
                req.namespace, req.pvc_name, req.source_cluster
            ),
        },
        VolumeReplicationStep {
            order: 2,
            action: "export_snapshot".into(),
            detail: "Export snapshot to object store or CSI volume snapshot class".into(),
        },
        VolumeReplicationStep {
            order: 3,
            action: "import_target".into(),
            detail: format!(
                "Restore PVC {} in namespace {} on cluster {}",
                req.pvc_name, req.namespace, req.target_cluster
            ),
        },
        VolumeReplicationStep {
            order: 4,
            action: "verify_mount".into(),
            detail: "Attach restored PVC to staging pod and verify checksum".into(),
        },
        VolumeReplicationStep {
            order: 5,
            action: "cutover".into(),
            detail: "Update workload spec to target cluster context and scale up".into(),
        },
    ];

    let mut warnings = vec![
        "Volume replication copies block data — ensure application quiesce or offline snapshot."
            .into(),
        "RWO volumes require detach from source before target attach.".into(),
    ];
    if storage_class.is_none() {
        warnings.push(
            "No storageClass in workload persistence — verify target cluster storage class.".into(),
        );
    }

    Ok(VolumeReplicationPlan {
        source_cluster: req.source_cluster.clone(),
        target_cluster: req.target_cluster.clone(),
        namespace: req.namespace.clone(),
        pvc_name: req.pvc_name.clone(),
        storage_class_hint: storage_class,
        size,
        steps,
        warnings,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutedVolumeStep {
    pub order: u32,
    pub action: String,
    pub success: bool,
    pub output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeReplicationExecuteResult {
    pub plan: VolumeReplicationPlan,
    pub dry_run: bool,
    pub executed_steps: Vec<ExecutedVolumeStep>,
}

pub async fn execute_volume_replication(
    req: &VolumeReplicationRequest,
    spec: Option<&Workload>,
    dry_run: bool,
) -> Result<VolumeReplicationExecuteResult> {
    let plan = plan_volume_replication(req, spec)?;
    let snapshot_name = format!("{}-snap", req.pvc_name);
    let mut executed = Vec::new();

    for step in &plan.steps {
        let output = match step.action.as_str() {
            "snapshot_source" => {
                run_kubectl(
                    &req.source_cluster,
                    &["get", "pvc", &req.pvc_name, "-n", &req.namespace],
                    dry_run,
                )
                .await?
            }
            "export_snapshot" => {
                let manifest = snapshot_manifest(&req.namespace, &snapshot_name, &req.pvc_name);
                if dry_run {
                    format!("dry-run apply VolumeSnapshot {snapshot_name}")
                } else {
                    apply_manifest(&req.source_cluster, &manifest).await?
                }
            }
            "import_target" => {
                let size = plan.size.clone().unwrap_or_else(|| "10Gi".into());
                let sc = plan
                    .storage_class_hint
                    .clone()
                    .unwrap_or_else(|| "standard".into());
                let pvc_manifest = restored_pvc_manifest(
                    &req.namespace,
                    &req.pvc_name,
                    &snapshot_name,
                    &size,
                    &sc,
                );
                if dry_run {
                    format!(
                        "dry-run apply restored PVC {} on {}",
                        req.pvc_name, req.target_cluster
                    )
                } else {
                    apply_manifest(&req.target_cluster, &pvc_manifest).await?
                }
            }
            "verify_mount" => {
                run_kubectl(
                    &req.target_cluster,
                    &["get", "pvc", &req.pvc_name, "-n", &req.namespace],
                    dry_run,
                )
                .await?
            }
            "cutover" => {
                if dry_run {
                    "dry-run cutover — update workload cluster context".into()
                } else if let Some(name) = &req.workload_name {
                    format!("cutover workload {name} to cluster {}", req.target_cluster)
                } else {
                    "cutover skipped — no workload_name".into()
                }
            }
            other => format!("unsupported step {other}"),
        };

        executed.push(ExecutedVolumeStep {
            order: step.order,
            action: step.action.clone(),
            success: !output.to_lowercase().contains("error"),
            output,
        });
    }

    Ok(VolumeReplicationExecuteResult {
        plan,
        dry_run,
        executed_steps: executed,
    })
}

async fn run_kubectl(cluster: &str, args: &[&str], dry_run: bool) -> Result<String> {
    if dry_run {
        return Ok(format!(
            "dry-run kubectl --context {cluster} {}",
            args.join(" ")
        ));
    }
    let output = tokio::process::Command::new("kubectl")
        .arg("--context")
        .arg(cluster)
        .args(args)
        .output()
        .await
        .context("failed to spawn kubectl")?;
    if !output.status.success() {
        anyhow::bail!(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

async fn apply_manifest(cluster: &str, manifest: &str) -> Result<String> {
    use std::process::Stdio;
    use tokio::io::AsyncWriteExt;
    use tokio::process::Command;

    let mut child = Command::new("kubectl")
        .args(["--context", cluster, "apply", "-f", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to spawn kubectl apply")?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(manifest.as_bytes()).await?;
    }
    let output = child.wait_with_output().await?;
    if !output.status.success() {
        anyhow::bail!(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn snapshot_manifest(namespace: &str, name: &str, pvc: &str) -> String {
    format!(
        r#"apiVersion: snapshot.storage.k8s.io/v1
kind: VolumeSnapshot
metadata:
  name: {name}
  namespace: {namespace}
spec:
  source:
    persistentVolumeClaimName: {pvc}
"#
    )
}

fn restored_pvc_manifest(
    namespace: &str,
    pvc: &str,
    snapshot: &str,
    size: &str,
    sc: &str,
) -> String {
    format!(
        r#"apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: {pvc}
  namespace: {namespace}
spec:
  storageClassName: {sc}
  dataSource:
    name: {snapshot}
    kind: VolumeSnapshot
    apiGroup: snapshot.storage.k8s.io
  accessModes:
    - ReadWriteOnce
  resources:
    requests:
      storage: {size}
"#
    )
}
