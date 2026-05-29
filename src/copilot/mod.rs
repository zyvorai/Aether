// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! AI Ops Copilot — natural language control plane with tool calling.

pub mod agent;
pub mod diagnose;
pub mod policy;
pub mod provider;
pub mod session;
pub mod tools;

pub use agent::CopilotAgent;
pub use session::CopilotSessionStore;
