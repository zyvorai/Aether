// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! `aether timeline <name>`.

use aether::ops::{self, board, timeline};
use aether::output;
use aether::state::StateStore;
use anyhow::Result;

pub(crate) fn timeline_command(
    name: &str,
    since: &str,
    only: Option<&str>,
    limit: usize,
) -> Result<()> {
    ops::check_name(name)?;
    let now = chrono::Utc::now();
    // History outlives the workload, so it need not be in the state file.
    let runtime = StateStore::load(&StateStore::default_path())?
        .get(name)
        .map(|w| w.runtime);
    let t = timeline::collect(
        &ops::data_dir(),
        &timeline::Query {
            workload: name.to_string(),
            runtime,
            since: ops::parse_since(since, now)?,
            now,
            only: timeline::parse_categories(only.unwrap_or_default())?,
            limit: limit.max(1),
            gap_secs: board::stale_after_secs(board::observe_secs_from_env()),
        },
    )?;
    if !t.known && t.unavailable.is_empty() {
        anyhow::bail!(
            "Nothing is recorded about '{name}'.\nHint: Run `aether list` to see deployed workloads."
        );
    }
    if output::is_json() {
        println!("{}", serde_json::to_string_pretty(&t)?);
    } else if output::is_yaml() {
        print!("{}", serde_yaml::to_string(&t)?);
    } else {
        print!("{}", timeline::format(&t));
    }
    Ok(())
}
