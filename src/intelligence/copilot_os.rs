// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Deprecated — re-exports from zeus_os.
pub use super::zeus_os::*;

// Legacy type aliases
pub type CopilotMemoryEntry = crate::zeus::memory::ZeusMemoryEntry;
pub type CopilotMemoryReport = crate::zeus::memory::ZeusMemoryReport;
pub type CopilotAuditEntry = super::zeus_os::ZeusAuditEntry;
pub type CopilotAuditReport = super::zeus_os::ZeusAuditReport;
pub type CopilotRbacScopesReport = super::zeus_os::ZeusRbacScopesReport;
pub type CopilotToolScope = super::zeus_os::ZeusToolScope;
pub type VoiceCopilotLabReport = super::zeus_os::VoiceZeusLabReport;

pub fn read_copilot_memory() -> super::zeus_os::ZeusMemoryReport {
    crate::zeus::memory::read_zeus_memory()
}

pub fn write_copilot_memory_entry(
    entry: CopilotMemoryEntry,
) -> anyhow::Result<CopilotMemoryReport> {
    crate::zeus::memory::write_zeus_memory_entry(entry)
}

pub fn append_copilot_audit(entry: CopilotAuditEntry) -> anyhow::Result<()> {
    super::zeus_os::append_zeus_audit(entry)
}

pub fn read_copilot_audit(limit: usize) -> CopilotAuditReport {
    super::zeus_os::read_zeus_audit(limit)
}

pub fn route_copilot_agent(message: &str) -> super::zeus_os::MultiAgentRouteReport {
    super::zeus_os::route_zeus_agent(message)
}

pub fn build_copilot_rbac_scopes(role: crate::rbac::Role) -> CopilotRbacScopesReport {
    super::zeus_os::build_zeus_rbac_scopes(role)
}

pub fn build_voice_copilot_lab() -> VoiceCopilotLabReport {
    super::zeus_os::build_voice_zeus_lab()
}
