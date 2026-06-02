// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! PostgreSQL backing store for API workload state (`StateStore`).
//!
//! Set `AETHER_STATE_DATABASE_URL` (libpq-style URI, e.g. `postgres://user:pass@host:5432/db`) when
//! running multiple `aether serve` replicas so all instances share the same workload JSON.
//! The on-disk file at `state_path` is still written on each persist for local tooling and
//! migration subprocesses that read the file.
//!
//! Optional: `AETHER_STATE_POLL_SECS` — how often replicas poll Postgres for remote changes (default `2`).

use crate::state::StateStore;
use anyhow::Context;
use deadpool_postgres::{Config, Pool, Runtime};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_postgres::NoTls;

const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS aether_workload_state (
    id text PRIMARY KEY,
    version bigint NOT NULL DEFAULT 0,
    body jsonb NOT NULL,
    updated_at timestamptz NOT NULL DEFAULT now()
);
INSERT INTO aether_workload_state (id, version, body) VALUES ('global', 0, '{}'::jsonb)
ON CONFLICT (id) DO NOTHING;
"#;

#[derive(Clone)]
pub struct WorkloadStatePool {
    pool: Pool,
}

impl WorkloadStatePool {
    pub async fn connect(database_url: &str) -> anyhow::Result<Self> {
        let cfg = Config {
            url: Some(database_url.to_string()),
            ..Default::default()
        };
        let pool = cfg
            .create_pool(Some(Runtime::Tokio1), NoTls)
            .context("failed to create Postgres pool for workload state")?;
        let s = Self { pool };
        s.ensure_schema().await?;
        Ok(s)
    }

    async fn ensure_schema(&self) -> anyhow::Result<()> {
        let c = self
            .pool
            .get()
            .await
            .context("postgres pool get (schema)")?;
        c.batch_execute(SCHEMA_SQL)
            .await
            .context("create aether_workload_state table")?;
        Ok(())
    }

    /// If Postgres has only an empty document, copy from the local state file once.
    pub async fn bootstrap_from_file_if_empty(&self, path: &Path) -> anyhow::Result<()> {
        let c = self
            .pool
            .get()
            .await
            .context("postgres pool get (bootstrap)")?;
        let row = c
            .query_opt(
                "SELECT body FROM aether_workload_state WHERE id = 'global'",
                &[],
            )
            .await
            .context("bootstrap select workload state")?;
        let pg_nonempty = row
            .map(|r| {
                let v: serde_json::Value = r.get(0);
                serde_json::from_value::<StateStore>(v)
                    .map(|st| !st.workloads.is_empty())
                    .unwrap_or(false)
            })
            .unwrap_or(false);
        if pg_nonempty {
            return Ok(());
        }
        if path.exists() {
            let st = StateStore::load(path)?;
            if st.workloads.is_empty() {
                return Ok(());
            }
            let v = serde_json::to_value(&st).context("serialize state for bootstrap")?;
            c.execute(
                "INSERT INTO aether_workload_state (id, version, body) VALUES ('global', 0, $1)
                 ON CONFLICT (id) DO UPDATE SET body = EXCLUDED.body, version = 0, updated_at = now()",
                &[&v],
            )
            .await
            .context("bootstrap insert workload state")?;
        }
        Ok(())
    }

    pub async fn load(&self) -> anyhow::Result<StateStore> {
        let c = self.pool.get().await.context("postgres pool get (load)")?;
        let row = c
            .query_opt(
                "SELECT body FROM aether_workload_state WHERE id = 'global'",
                &[],
            )
            .await
            .context("select workload state body")?;
        match row {
            None => Ok(StateStore::new()),
            Some(r) => {
                let v: serde_json::Value = r.get(0);
                serde_json::from_value(v).context("deserialize StateStore from postgres jsonb")
            }
        }
    }

    pub async fn ping_ok(&self) -> bool {
        match self.pool.get().await {
            Ok(c) => c.query_one("SELECT 1", &[]).await.is_ok(),
            Err(_) => false,
        }
    }

    pub async fn save(&self, store: &StateStore) -> anyhow::Result<()> {
        let v = serde_json::to_value(store).context("serialize StateStore for postgres")?;
        let c = self.pool.get().await.context("postgres pool get (save)")?;
        c.execute(
            "INSERT INTO aether_workload_state (id, version, body) VALUES ('global', 1, $1)
             ON CONFLICT (id) DO UPDATE SET
               body = EXCLUDED.body,
               version = aether_workload_state.version + 1,
               updated_at = now()",
            &[&v],
        )
        .await
        .context("upsert workload state in postgres")?;
        Ok(())
    }
}

/// Persist workload state to Postgres (if configured) and to the local JSON file.
pub async fn persist_workload_state(
    pg: Option<&Arc<WorkloadStatePool>>,
    state_path: &Path,
    store: &StateStore,
) -> anyhow::Result<()> {
    if let Some(pool) = pg {
        pool.save(store).await?;
    }
    store.save(state_path)
}

/// Poll shared state so other API replicas' writes become visible (last-write-wins JSON snapshot).
pub fn spawn_workload_state_poller(pool: Arc<WorkloadStatePool>, shared: Arc<RwLock<StateStore>>) {
    let secs = std::env::var("AETHER_STATE_POLL_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|n| *n > 0 && *n <= 3600)
        .unwrap_or(2);

    tokio::spawn(async move {
        let mut last = String::new();
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(secs));
        loop {
            tick.tick().await;
            match pool.load().await {
                Ok(remote) => {
                    let Ok(j) = serde_json::to_string(&remote) else {
                        continue;
                    };
                    if j != last {
                        last = j;
                        *shared.write().await = remote;
                    }
                }
                Err(e) => tracing::warn!(error = %e, "workload state poll from postgres failed"),
            }
        }
    });
}
