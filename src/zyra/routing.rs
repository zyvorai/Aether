// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Zyra intelligent routing — task classification and provider/agent selection.

use crate::zyra::providers::{ZyraProviderKind, ZyraProviderRegistry};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskClass {
    Infrastructure,
    CodeGeneration,
    SecurityAnalysis,
    Research,
    LongContext,
    FastResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    pub agent_id: String,
    pub agent_label: String,
    pub task_class: TaskClass,
    pub provider_kind: ZyraProviderKind,
    pub model_hint: String,
    pub confidence: f64,
    pub rationale: String,
    pub suggested_prompts: Vec<String>,
}

pub fn classify_task(message: &str) -> TaskClass {
    let lower = message.to_lowercase();
    if lower.contains("security") || lower.contains("threat") || lower.contains("vulnerability") {
        return TaskClass::SecurityAnalysis;
    }
    if lower.contains("terraform")
        || lower.contains("ansible")
        || lower.contains("generate")
        || lower.contains("code")
    {
        return TaskClass::CodeGeneration;
    }
    if lower.contains("research")
        || lower.contains("compare")
        || lower.contains("finops")
        || lower.contains("cost")
    {
        return TaskClass::Research;
    }
    if lower.contains("log")
        || lower.contains("trace")
        || lower.contains("metric")
        || lower.contains("history")
    {
        return TaskClass::LongContext;
    }
    if lower.contains("quick") || lower.contains("fast") || lower.contains("local") {
        return TaskClass::FastResponse;
    }
    TaskClass::Infrastructure
}

pub fn route_agent(
    message: &str,
    agent_override: Option<&str>,
) -> (String, String, f64, Vec<String>) {
    if let Some(id) = agent_override.filter(|s| !s.is_empty() && *s != "auto") {
        let persona = crate::zyra::agents::get_agent(id);
        return (persona.id, persona.label, 0.95, persona.suggested_prompts);
    }
    let lower = message.to_lowercase();
    if lower.contains("architect") || lower.contains("design") || lower.contains("placement") {
        let p = crate::zyra::agents::get_agent("architect");
        return (p.id, p.label, 0.9, p.suggested_prompts);
    }
    if lower.contains("devops")
        || lower.contains("ci/cd")
        || lower.contains("pipeline")
        || lower.contains("gitops")
        || lower.contains("drift")
    {
        let p = crate::zyra::agents::get_agent("devops");
        return (p.id, p.label, 0.88, p.suggested_prompts);
    }
    if lower.contains("kubernetes")
        || lower.contains("k8s")
        || lower.contains("pod")
        || lower.contains("cluster")
    {
        let p = crate::zyra::agents::get_agent("kubernetes");
        return (p.id, p.label, 0.87, p.suggested_prompts);
    }
    if lower.contains("security") || lower.contains("threat") || lower.contains("policy") {
        let p = crate::zyra::agents::get_agent("security");
        return (p.id, p.label, 0.9, p.suggested_prompts);
    }
    if lower.contains("cost") || lower.contains("finops") || lower.contains("savings") {
        let p = crate::zyra::agents::get_agent("cost");
        return (p.id, p.label, 0.92, p.suggested_prompts);
    }
    if lower.contains("observability")
        || lower.contains("log")
        || lower.contains("metric")
        || lower.contains("trace")
    {
        let p = crate::zyra::agents::get_agent("observability");
        return (p.id, p.label, 0.86, p.suggested_prompts);
    }
    if lower.contains("database")
        || lower.contains("postgres")
        || lower.contains("mysql")
        || lower.contains("redis")
    {
        let p = crate::zyra::agents::get_agent("database");
        return (p.id, p.label, 0.85, p.suggested_prompts);
    }
    if lower.contains("llm")
        || lower.contains("gpu")
        || lower.contains("inference")
        || lower.contains("model")
    {
        let p = crate::zyra::agents::get_agent("ai_engineer");
        return (p.id, p.label, 0.84, p.suggested_prompts);
    }
    let p = crate::zyra::agents::get_agent("sre");
    (p.id, p.label, 0.75, p.suggested_prompts)
}

pub fn preferred_provider_kind(task: TaskClass) -> ZyraProviderKind {
    match task {
        TaskClass::Infrastructure => ZyraProviderKind::Anthropic,
        TaskClass::CodeGeneration => ZyraProviderKind::Openai,
        TaskClass::SecurityAnalysis => ZyraProviderKind::Openai,
        TaskClass::Research => ZyraProviderKind::Gemini,
        TaskClass::LongContext => ZyraProviderKind::Anthropic,
        TaskClass::FastResponse => ZyraProviderKind::Ollama,
    }
}

pub fn route_message(message: &str, agent_override: Option<&str>) -> RoutingDecision {
    let task = classify_task(message);
    let (agent_id, agent_label, confidence, suggested_prompts) =
        route_agent(message, agent_override);
    let provider_kind = preferred_provider_kind(task);
    let model_hint = match provider_kind {
        ZyraProviderKind::Anthropic => "claude-sonnet-4-20250514".into(),
        ZyraProviderKind::Openai => "gpt-4o-mini".into(),
        ZyraProviderKind::Gemini => "gemini-2.0-flash".into(),
        ZyraProviderKind::Xai => "grok-2".into(),
        ZyraProviderKind::Ollama => "llama3.2".into(),
        _ => "default".into(),
    };
    let rationale = format!("Classified as {:?}, routed to {}", task, agent_label);
    RoutingDecision {
        agent_id,
        agent_label,
        task_class: task,
        provider_kind,
        model_hint,
        confidence,
        rationale,
        suggested_prompts,
    }
}

pub fn resolve_provider_for_routing(
    decision: &RoutingDecision,
    reg: &ZyraProviderRegistry,
) -> ZyraProviderKind {
    let preferred = decision.provider_kind;
    if reg
        .providers
        .iter()
        .any(|p| p.enabled && p.kind == preferred)
    {
        return preferred;
    }
    reg.providers
        .iter()
        .find(|p| p.enabled)
        .map(|p| p.kind)
        .unwrap_or(ZyraProviderKind::Openai)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_security_to_security_agent() {
        let d = route_message("Scan fleet for security threats", None);
        assert_eq!(d.agent_id, "security");
    }

    #[test]
    fn routes_cost_to_cost_agent() {
        let d = route_message("Find cost savings", None);
        assert_eq!(d.agent_id, "cost");
    }
}
