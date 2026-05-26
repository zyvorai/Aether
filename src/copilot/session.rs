// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Copilot session store.

use crate::copilot::provider::ChatMessage;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopilotSession {
    pub id: String,
    pub messages: Vec<ChatMessage>,
    pub pending_actions: Vec<PendingAction>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingAction {
    pub id: String,
    pub tool: String,
    pub arguments: serde_json::Value,
    pub description: String,
}

#[derive(Clone, Default)]
pub struct CopilotSessionStore {
    inner: Arc<Mutex<HashMap<String, CopilotSession>>>,
}

impl CopilotSessionStore {
    pub fn get_or_create(&self, session_id: Option<&str>) -> CopilotSession {
        let mut map = self.inner.lock().unwrap();
        let id = session_id
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("sess-{}", uuid_simple()));
        if let Some(sess) = map.get(&id) {
            return sess.clone();
        }
        let now = crate::resources::now_rfc3339();
        let sess = CopilotSession {
            id: id.clone(),
            messages: Vec::new(),
            pending_actions: Vec::new(),
            created_at: now.clone(),
            updated_at: now,
        };
        map.insert(id, sess.clone());
        sess
    }

    pub fn save(&self, session: CopilotSession) {
        let mut map = self.inner.lock().unwrap();
        map.insert(session.id.clone(), session);
    }

    pub fn get(&self, id: &str) -> Option<CopilotSession> {
        self.inner.lock().unwrap().get(id).cloned()
    }
}

fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{n:x}")
}
