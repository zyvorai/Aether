// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

//! Zyra specialist agent personas.

use crate::zyra::routing::TaskClass;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZyraAgentPersona {
    pub id: String,
    pub label: String,
    pub description: String,
    pub task_class: TaskClass,
    pub system_prompt: String,
    pub suggested_prompts: Vec<String>,
    pub tools: Vec<String>,
}

pub fn all_agents() -> Vec<ZyraAgentPersona> {
    vec![
        persona(
            "auto",
            "Auto",
            "Zyra automatically selects the best specialist.",
            TaskClass::Infrastructure,
            "You are Zyra, the AI infrastructure operating layer. Route tasks to the right tools.",
            vec![
                "Summarize fleet health".into(),
                "What needs attention today?".into(),
            ],
            vec!["*"],
        ),
        persona(
            "architect",
            "Zyra Architect",
            "Designs infrastructure and runtime placement.",
            TaskClass::Infrastructure,
            "You are Zyra Architect. Design optimal infrastructure, placement, and migration paths.",
            vec![
                "Recommend runtime placement".into(),
                "Design a multi-region architecture".into(),
            ],
            vec!["recommend_runtime", "intelligence_place", "cluster_summary"],
        ),
        persona(
            "devops",
            "Zyra DevOps",
            "Handles CI/CD, GitOps, and deployments.",
            TaskClass::CodeGeneration,
            "You are Zyra DevOps. Focus on GitOps, drift, deployments, and CI/CD pipelines.",
            vec![
                "Check GitOps drift".into(),
                "Generate deployment plan".into(),
            ],
            vec!["gitops_status", "check_drift", "deploy_spec", "generate_artifact"],
        ),
        persona(
            "kubernetes",
            "Zyra Kubernetes",
            "Manages clusters, pods, and workloads.",
            TaskClass::Infrastructure,
            "You are Zyra Kubernetes. Diagnose cluster issues, pods, and workload health.",
            vec![
                "Why are pods crashing?".into(),
                "Show cluster summary".into(),
            ],
            vec!["diagnose_workload", "cluster_summary", "explain_health"],
        ),
        persona(
            "security",
            "Zyra Security",
            "Threat detection and policy analysis.",
            TaskClass::SecurityAnalysis,
            "You are Zyra Security. Scan threats, policy violations, and attestation failures.",
            vec![
                "Scan fleet for threats".into(),
                "Explain policy violations".into(),
            ],
            vec!["threats", "policy_violations", "trust_score_fleet"],
        ),
        persona(
            "sre",
            "Zyra SRE",
            "Incident response and reliability.",
            TaskClass::Infrastructure,
            "You are Zyra SRE. Diagnose incidents, health issues, and healing actions.",
            vec![
                "Run fleet health diagnosis".into(),
                "Which workloads need healing?".into(),
            ],
            vec!["explain_health", "diagnose_workload", "predictions", "restart_workload"],
        ),
        persona(
            "cost",
            "Zyra Cost Optimizer",
            "Cloud cost analysis and FinOps.",
            TaskClass::Research,
            "You are Zyra Cost Optimizer. Find savings, chargeback, and placement cost wins.",
            vec![
                "Find workloads wasting resources".into(),
                "Predict cost next month".into(),
            ],
            vec!["cost_summary", "ai_insights"],
        ),
        persona(
            "observability",
            "Zyra Observability",
            "Logs, metrics, and traces.",
            TaskClass::LongContext,
            "You are Zyra Observability. Analyze metrics, logs, events, and SLA violations.",
            vec![
                "Query latency metrics".into(),
                "Show recent critical events".into(),
            ],
            vec!["query_metrics", "context_snapshot", "ai_insights"],
        ),
        persona(
            "ai_engineer",
            "Zyra AI Engineer",
            "LLM deployment and confidential compute.",
            TaskClass::CodeGeneration,
            "You are Zyra AI Engineer. GPU placement, confidential migration, and inference workloads.",
            vec![
                "Plan confidential migration".into(),
                "Score GPU placement".into(),
            ],
            vec!["confidential_migrate_plan", "trust_score_fleet", "intelligence_place"],
        ),
        persona(
            "database",
            "Zyra Database Expert",
            "Database optimization and scaling.",
            TaskClass::Infrastructure,
            "You are Zyra Database Expert. Optimize database workloads, resources, and scaling.",
            vec![
                "Recommend database scaling".into(),
                "Analyze resource saturation".into(),
            ],
            vec!["explain_health", "predictions", "query_metrics"],
        ),
    ]
}

fn persona(
    id: &str,
    label: &str,
    description: &str,
    task_class: TaskClass,
    system_prompt: &str,
    suggested_prompts: Vec<String>,
    tools: Vec<&str>,
) -> ZyraAgentPersona {
    ZyraAgentPersona {
        id: id.into(),
        label: label.into(),
        description: description.into(),
        task_class,
        system_prompt: system_prompt.into(),
        suggested_prompts,
        tools: tools.into_iter().map(String::from).collect(),
    }
}

pub fn get_agent(id: &str) -> ZyraAgentPersona {
    all_agents()
        .into_iter()
        .find(|a| a.id == id)
        .unwrap_or_else(|| get_agent("sre"))
}

pub fn agent_system_prompt(id: &str) -> String {
    get_agent(id).system_prompt
}
