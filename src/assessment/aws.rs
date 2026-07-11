// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! AWS managed-service detection and Kubernetes-native migration mapping.
//!
//! Discovered application endpoints (env values) are matched against AWS service
//! signatures. Each detected dependency is mapped to a Kubernetes-native target
//! (aligned with the Zyvor stack — CloudNativePG/Percona, Ceph RGW, Strimzi, …)
//! and a migration method, so a full AWS→Kubernetes plan can cover *all* services.

use crate::inventory::application::Application;
use serde::{Deserialize, Serialize};

/// A recognised AWS managed service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AwsService {
    Rds,
    Aurora,
    ElastiCache,
    S3,
    Sqs,
    Sns,
    DynamoDb,
    OpenSearch,
    Msk,
    SecretsManager,
    Kms,
    Ecr,
    ApiGateway,
    Elb,
}

impl AwsService {
    pub fn name(&self) -> &'static str {
        match self {
            AwsService::Rds => "Amazon RDS",
            AwsService::Aurora => "Amazon Aurora",
            AwsService::ElastiCache => "Amazon ElastiCache",
            AwsService::S3 => "Amazon S3",
            AwsService::Sqs => "Amazon SQS",
            AwsService::Sns => "Amazon SNS",
            AwsService::DynamoDb => "Amazon DynamoDB",
            AwsService::OpenSearch => "Amazon OpenSearch",
            AwsService::Msk => "Amazon MSK (Kafka)",
            AwsService::SecretsManager => "AWS Secrets Manager",
            AwsService::Kms => "AWS KMS",
            AwsService::Ecr => "Amazon ECR",
            AwsService::ApiGateway => "Amazon API Gateway",
            AwsService::Elb => "AWS Elastic Load Balancer",
        }
    }

    /// The Kubernetes-native replacement Aether recommends.
    pub fn k8s_target(&self) -> &'static str {
        match self {
            AwsService::Rds | AwsService::Aurora => "CloudNativePG / Percona on Ceph (via DataBridge)",
            AwsService::ElastiCache => "Redis / KeyDB (Bitnami/operator)",
            AwsService::S3 => "Ceph RGW / MinIO (S3-compatible, via Atlas)",
            AwsService::Sqs => "NATS JetStream / RabbitMQ",
            AwsService::Sns => "NATS / Knative Eventing",
            AwsService::DynamoDb => "ScyllaDB / Cassandra",
            AwsService::OpenSearch => "OpenSearch operator",
            AwsService::Msk => "Strimzi Kafka",
            AwsService::SecretsManager => "HashiCorp Vault + External Secrets",
            AwsService::Kms => "Vault Transit / cluster KMS",
            AwsService::Ecr => "Harbor registry",
            AwsService::ApiGateway => "Ingress / Gateway API",
            AwsService::Elb => "Service LoadBalancer / MetalLB + Ingress",
        }
    }

    /// The recommended migration method for this class of service.
    pub fn method(&self) -> &'static str {
        match self {
            AwsService::Rds | AwsService::Aurora => {
                "Logical replication (Debezium CDC) → cutover; verify row counts"
            }
            AwsService::ElastiCache => "Warm cache / replication; caches can be rebuilt on cutover",
            AwsService::S3 => "Bucket sync (rclone/mc mirror) → final delta at cutover",
            AwsService::Sqs | AwsService::Sns => "Drain queues, dual-write during cutover",
            AwsService::DynamoDb => "Export/import or dual-write; schema remap",
            AwsService::OpenSearch => "Snapshot/restore or reindex",
            AwsService::Msk => "MirrorMaker 2 topic replication",
            AwsService::SecretsManager | AwsService::Kms => {
                "Re-create secrets in Vault; rotate; never copy plaintext"
            }
            AwsService::Ecr => "Mirror images (skopeo) to the target registry, rewrite refs",
            AwsService::ApiGateway => "Recreate routes as Ingress/Gateway resources",
            AwsService::Elb => "Recreate as Service type=LoadBalancer / Ingress",
        }
    }

    /// Rough migration difficulty (feeds project risk).
    pub fn difficulty(&self) -> &'static str {
        match self {
            AwsService::S3
            | AwsService::Ecr
            | AwsService::Elb
            | AwsService::ApiGateway
            | AwsService::SecretsManager
            | AwsService::Kms => "low",
            AwsService::ElastiCache | AwsService::Sqs | AwsService::Sns | AwsService::OpenSearch => {
                "medium"
            }
            AwsService::Rds | AwsService::Aurora | AwsService::DynamoDb | AwsService::Msk => "high",
        }
    }
}

/// A detected AWS dependency of an application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsDependency {
    pub application: String,
    pub service: AwsService,
    pub endpoint: String,
}

/// True when `e` is a regional AWS service host `<service>.<region>.amazonaws.com`.
fn aws_service_host(e: &str, service: &str) -> bool {
    e.contains("amazonaws.com")
        && (e.contains(&format!("//{service}.")) || e.contains(&format!(".{service}.")))
}

/// Match an endpoint against AWS service signatures.
fn classify_endpoint(e: &str) -> Option<AwsService> {
    let e = e.to_lowercase();
    // Order matters: more specific first.
    if e.contains(".cache.amazonaws.com") {
        Some(AwsService::ElastiCache)
    } else if e.contains(".rds.amazonaws.com") {
        if e.contains(".cluster-") || e.contains("aurora") {
            Some(AwsService::Aurora)
        } else {
            Some(AwsService::Rds)
        }
    } else if e.contains(".s3.") || e.contains("s3.amazonaws.com") || e.contains(".s3-") {
        Some(AwsService::S3)
    } else if aws_service_host(&e, "sqs") {
        Some(AwsService::Sqs)
    } else if aws_service_host(&e, "sns") {
        Some(AwsService::Sns)
    } else if aws_service_host(&e, "dynamodb") {
        Some(AwsService::DynamoDb)
    } else if e.contains(".es.amazonaws.com") || e.contains(".aoss.") || e.contains("opensearch") {
        Some(AwsService::OpenSearch)
    } else if e.contains(".kafka.") || e.contains("msk") {
        Some(AwsService::Msk)
    } else if aws_service_host(&e, "secretsmanager") {
        Some(AwsService::SecretsManager)
    } else if aws_service_host(&e, "kms") {
        Some(AwsService::Kms)
    } else if e.contains(".dkr.ecr.") {
        Some(AwsService::Ecr)
    } else if e.contains(".execute-api.") {
        Some(AwsService::ApiGateway)
    } else if e.contains(".elb.amazonaws.com") || e.contains(".elb.") {
        Some(AwsService::Elb)
    } else {
        None
    }
}

/// Detect all AWS dependencies across a set of applications.
pub fn detect(apps: &[Application]) -> Vec<AwsDependency> {
    let mut deps = Vec::new();
    for app in apps {
        for e in &app.external_deps {
            if let Some(service) = classify_endpoint(e) {
                deps.push(AwsDependency {
                    application: app.name.clone(),
                    service,
                    endpoint: e.clone(),
                });
            }
        }
    }
    deps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_endpoints() {
        assert_eq!(
            classify_endpoint("mydb.abc.us-east-1.rds.amazonaws.com:5432"),
            Some(AwsService::Rds)
        );
        assert_eq!(
            classify_endpoint("app.cluster-xyz.us-east-1.rds.amazonaws.com"),
            Some(AwsService::Aurora)
        );
        assert_eq!(
            classify_endpoint("redis.abc.cache.amazonaws.com:6379"),
            Some(AwsService::ElastiCache)
        );
        assert_eq!(
            classify_endpoint("https://my-bucket.s3.us-east-1.amazonaws.com"),
            Some(AwsService::S3)
        );
        assert_eq!(
            classify_endpoint("123.dkr.ecr.us-east-1.amazonaws.com/app"),
            Some(AwsService::Ecr)
        );
        assert_eq!(classify_endpoint("https://api.stripe.com"), None);
    }

    #[test]
    fn test_detect_across_apps() {
        let mut app = Application {
            id: "p/web".into(),
            name: "web".into(),
            namespace: "p".into(),
            connection: "c".into(),
            workloads: vec![],
            services: vec![],
            pvcs: vec![],
            config_map_refs: vec![],
            secret_refs: vec![],
            external_deps: vec![
                "db.abc.us-east-1.rds.amazonaws.com:5432".into(),
                "https://assets.s3.amazonaws.com".into(),
                "https://api.stripe.com".into(),
            ],
            class: crate::inventory::application::MigrationClass::CloudManagedDependency,
            blockers: vec![],
            warnings: vec![],
        };
        app.name = "web".into();
        let deps = detect(&[app]);
        assert_eq!(deps.len(), 2); // stripe is not AWS
        assert!(deps.iter().any(|d| d.service == AwsService::Rds));
        assert!(deps.iter().any(|d| d.service == AwsService::S3));
    }
}
