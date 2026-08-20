// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Deprecated — re-exports from zyra_os.
pub use super::zyra_os::*;

// Legacy type aliases
pub type CopilotMemoryEntry = crate::zyra::memory::ZyraMemoryEntry;
pub type CopilotMemoryReport = crate::zyra::memory::ZyraMemoryReport;
pub type CopilotAuditEntry = super::zyra_os::ZyraAuditEntry;
pub type CopilotAuditReport = super::zyra_os::ZyraAuditReport;
pub type CopilotRbacScopesReport = super::zyra_os::ZyraRbacScopesReport;
pub type CopilotToolScope = super::zyra_os::ZyraToolScope;
pub type VoiceCopilotLabReport = super::zyra_os::VoiceZyraLabReport;

pub fn read_copilot_memory() -> super::zyra_os::ZyraMemoryReport {
    crate::zyra::memory::read_zyra_memory()
}

pub fn write_copilot_memory_entry(
    entry: CopilotMemoryEntry,
) -> anyhow::Result<CopilotMemoryReport> {
    crate::zyra::memory::write_zyra_memory_entry(entry)
}

pub fn append_copilot_audit(entry: CopilotAuditEntry) -> anyhow::Result<()> {
    super::zyra_os::append_zyra_audit(entry)
}

pub fn read_copilot_audit(limit: usize) -> CopilotAuditReport {
    super::zyra_os::read_zyra_audit(limit)
}

pub fn route_copilot_agent(message: &str) -> super::zyra_os::MultiAgentRouteReport {
    super::zyra_os::route_zyra_agent(message)
}

pub fn build_copilot_rbac_scopes(role: crate::rbac::Role) -> CopilotRbacScopesReport {
    super::zyra_os::build_zyra_rbac_scopes(role)
}

pub fn build_voice_copilot_lab() -> VoiceCopilotLabReport {
    super::zyra_os::build_voice_zyra_lab()
}
