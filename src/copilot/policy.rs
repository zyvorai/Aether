//! RBAC-aware copilot tool policies.

use crate::rbac::Role;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolRisk {
    Read,
    Mutate,
}

pub fn tool_risk(name: &str) -> ToolRisk {
    match name {
        "migrate_workload" | "restart_workload" | "apply_cluster_action" | "reconcile_drift" => {
            ToolRisk::Mutate
        }
        _ => ToolRisk::Read,
    }
}

pub fn role_allows_tool(role: &Role, risk: ToolRisk) -> bool {
    match (role, risk) {
        (_, ToolRisk::Read) => true,
        (Role::Admin, ToolRisk::Mutate) => true,
        (Role::Operator, ToolRisk::Mutate) => true,
        (Role::Viewer, ToolRisk::Mutate) => false,
    }
}

pub fn role_allows_execute(role: &Role, auto_execute: bool) -> bool {
    match role {
        Role::Admin => true,
        Role::Operator => auto_execute,
        Role::Viewer => false,
    }
}
