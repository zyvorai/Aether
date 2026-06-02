// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Cilium and Calico network policy CR builders.

use crate::spec::{CalicoPolicyRuleSpec, CiliumPolicyRuleSpec, Workload};
use serde_json::{json, Value};
use std::collections::BTreeMap;

fn workload_labels(spec: &Workload) -> BTreeMap<String, String> {
    let mut labels = BTreeMap::new();
    labels.insert("app".to_string(), spec.metadata.name.clone());
    labels.insert("managed-by".to_string(), "aether".to_string());
    labels
}

fn endpoint_selector(spec: &Workload) -> Value {
    json!({
        "matchLabels": {
            "app": spec.metadata.name,
            "managed-by": "aether"
        }
    })
}

fn calico_selector(spec: &Workload) -> String {
    format!("app == '{}' && managed-by == 'aether'", spec.metadata.name)
}

fn cilium_rule(rule: &CiliumPolicyRuleSpec) -> Value {
    let mut obj = json!({});
    if !rule.from_endpoints.is_empty() {
        obj["fromEndpoints"] = json!(rule
            .from_endpoints
            .iter()
            .map(|labels| json!({ "matchLabels": labels }))
            .collect::<Vec<_>>());
    }
    if !rule.to_endpoints.is_empty() {
        obj["toEndpoints"] = json!(rule
            .to_endpoints
            .iter()
            .map(|labels| json!({ "matchLabels": labels }))
            .collect::<Vec<_>>());
    }
    if !rule.from_cidr.is_empty() {
        obj["fromCIDR"] = json!(rule.from_cidr);
    }
    if !rule.to_cidr.is_empty() {
        obj["toCIDR"] = json!(rule.to_cidr);
    }
    if !rule.to_ports.is_empty() {
        obj["toPorts"] = json!(rule
            .to_ports
            .iter()
            .map(|p| {
                json!([{
                    "ports": [{ "port": p.port.to_string(), "protocol": p.protocol.to_uppercase() }]
                }])
            })
            .collect::<Vec<_>>());
    }
    obj
}

fn calico_rule(rule: &CalicoPolicyRuleSpec) -> Value {
    let mut obj = json!({ "action": rule.action });
    if let Some(sel) = &rule.source_selector {
        obj["source"] = json!({ "selector": sel });
    }
    let mut dest = json!({});
    if let Some(sel) = &rule.destination_selector {
        dest["selector"] = json!(sel);
    }
    if !rule.destination_nets.is_empty() {
        dest["nets"] = json!(rule.destination_nets);
    }
    if !rule.destination_ports.is_empty() {
        let proto = rule.protocol.as_deref().unwrap_or("TCP");
        dest["ports"] = json!(rule
            .destination_ports
            .iter()
            .map(|p| json!({ "protocol": proto, "port": p }))
            .collect::<Vec<_>>());
    } else if rule.protocol.is_some() {
        dest["protocol"] = json!(rule.protocol);
    }
    if dest.as_object().map(|o| !o.is_empty()).unwrap_or(false) {
        obj["destination"] = dest;
    }
    obj
}

pub(crate) fn build_cilium_network_policy_json(namespace: &str, spec: &Workload) -> Option<Value> {
    if let Some(cnp) = spec.network.cilium_network_policy.as_ref() {
        if cnp.enabled {
            return build_cilium_from_spec(namespace, spec, cnp);
        }
        return None;
    }
    let auto = crate::ragnarok::network::default_confidential_cilium(spec)?;
    build_cilium_from_spec(namespace, spec, &auto)
}

fn build_cilium_from_spec(
    namespace: &str,
    spec: &Workload,
    cnp: &crate::spec::CiliumNetworkPolicySpec,
) -> Option<Value> {
    let ingress: Vec<Value> = cnp.ingress.iter().map(cilium_rule).collect();
    let egress: Vec<Value> = cnp.egress.iter().map(cilium_rule).collect();
    let mut spec_body = json!({
        "endpointSelector": endpoint_selector(spec)
    });
    if !ingress.is_empty() {
        spec_body["ingress"] = json!(ingress);
    }
    if !egress.is_empty() {
        spec_body["egress"] = json!(egress);
    }
    Some(json!({
        "apiVersion": "cilium.io/v2",
        "kind": "CiliumNetworkPolicy",
        "metadata": {
            "name": format!("{}-cilium", spec.metadata.name),
            "namespace": namespace,
            "labels": workload_labels(spec)
        },
        "spec": spec_body
    }))
}

pub(crate) fn build_calico_network_policy_json(namespace: &str, spec: &Workload) -> Option<Value> {
    let calico = spec.network.calico_network_policy.as_ref()?;
    if !calico.enabled {
        return None;
    }
    let ingress: Vec<Value> = calico.ingress.iter().map(calico_rule).collect();
    let egress: Vec<Value> = calico.egress.iter().map(calico_rule).collect();
    let mut spec_body = json!({
        "selector": calico_selector(spec),
        "types": calico.types
    });
    if !ingress.is_empty() {
        spec_body["ingress"] = json!(ingress);
    }
    if !egress.is_empty() {
        spec_body["egress"] = json!(egress);
    }
    Some(json!({
        "apiVersion": "projectcalico.org/v3",
        "kind": "NetworkPolicy",
        "metadata": {
            "name": format!("{}-calico", spec.metadata.name),
            "namespace": namespace,
            "labels": workload_labels(spec)
        },
        "spec": spec_body
    }))
}

pub(crate) fn cilium_network_policy_api_resource() -> kube::api::ApiResource {
    kube::api::ApiResource {
        group: "cilium.io".into(),
        version: "v2".into(),
        api_version: "cilium.io/v2".into(),
        kind: "CiliumNetworkPolicy".into(),
        plural: "ciliumnetworkpolicies".into(),
    }
}

pub(crate) fn calico_network_policy_api_resource() -> kube::api::ApiResource {
    kube::api::ApiResource {
        group: "projectcalico.org".into(),
        version: "v3".into(),
        api_version: "projectcalico.org/v3".into(),
        kind: "NetworkPolicy".into(),
        plural: "networkpolicies".into(),
    }
}
