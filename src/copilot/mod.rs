//! AI Ops Copilot — natural language control plane with tool calling.

pub mod agent;
pub mod policy;
pub mod provider;
pub mod session;
pub mod tools;

pub use agent::CopilotAgent;
pub use session::CopilotSessionStore;
